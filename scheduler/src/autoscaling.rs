use anyhow::Result;
use aws_sdk_autoscaling::Client as AsgClient;
use aws_sdk_ec2::Client as Ec2Client;
use tracing::{error, info};

/// Suspend/resume handler for Auto Scaling Groups in a given AWS region.
///
/// Discovers ASGs by iterating through all groups and matching the given tag.
/// On stop: suspends ASG processes, then stops instances.
/// On start: starts instances, waits for them to be running, then resumes ASG processes.
pub struct AutoScalingScheduler {
    ec2: Ec2Client,
    asg: AsgClient,
}

impl AutoScalingScheduler {
    pub async fn new(region: &str) -> Self {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        Self {
            ec2: Ec2Client::new(&config),
            asg: AsgClient::new(&config),
        }
    }

    pub async fn stop(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let group_names = self.list_groups(tag_key, tag_value).await?;
        let instance_ids = self.list_instances(&group_names).await?;

        info!(
            groups = group_names.len(),
            instances = instance_ids.len(),
            "Found Auto Scaling resources to stop"
        );

        for name in &group_names {
            if let Err(e) = self.suspend_group(name).await {
                error!(group = %name, error = %e, "Failed to suspend ASG");
            }
        }

        for id in &instance_ids {
            if let Err(e) = self.stop_instance(id).await {
                error!(instance = %id, error = %e, "Failed to stop ASG instance");
            }
        }

        Ok(())
    }

    pub async fn start(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let group_names = self.list_groups(tag_key, tag_value).await?;
        let instance_ids = self.list_instances(&group_names).await?;

        info!(
            groups = group_names.len(),
            instances = instance_ids.len(),
            "Found Auto Scaling resources to start"
        );

        let mut started: Vec<String> = Vec::new();
        for id in &instance_ids {
            match self.start_instance(id).await {
                Ok(()) => started.push(id.clone()),
                Err(e) => error!(instance = %id, error = %e, "Failed to start ASG instance"),
            }
        }

        if !started.is_empty() {
            if let Err(e) = self.wait_instances_running(&started).await {
                error!(error = %e, "Error while waiting for instances to be running");
            }
        }

        for name in &group_names {
            if let Err(e) = self.resume_group(name).await {
                error!(group = %name, error = %e, "Failed to resume ASG");
            }
        }

        Ok(())
    }

    /// List Auto Scaling Group names matching the given tag by paginating
    /// through all groups and filtering manually.
    async fn list_groups(&self, tag_key: &str, tag_value: &str) -> Result<Vec<String>> {
        let mut names = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.asg.describe_auto_scaling_groups();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let resp = req.send().await?;

            for group in resp.auto_scaling_groups() {
                for tag in group.tags() {
                    if tag.key().unwrap_or_default() == tag_key
                        && tag.value().unwrap_or_default() == tag_value
                    {
                        if let Some(name) = group.auto_scaling_group_name() {
                            names.push(name.to_string());
                        }
                        break;
                    }
                }
            }

            match resp.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        info!(count = names.len(), "Found Auto Scaling groups with matching tag");
        Ok(names)
    }

    /// List all instance IDs belonging to the given Auto Scaling Groups.
    async fn list_instances(&self, group_names: &[String]) -> Result<Vec<String>> {
        if group_names.is_empty() {
            return Ok(Vec::new());
        }

        let mut ids = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.asg.describe_auto_scaling_groups();
            for name in group_names {
                req = req.auto_scaling_group_names(name);
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let resp = req.send().await?;

            for group in resp.auto_scaling_groups() {
                for instance in group.instances() {
                    if let Some(id) = instance.instance_id() {
                        ids.push(id.to_string());
                    }
                }
            }

            match resp.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(ids)
    }

    async fn suspend_group(&self, group_name: &str) -> Result<()> {
        info!(group = %group_name, "Suspending ASG processes");
        self.asg
            .suspend_processes()
            .auto_scaling_group_name(group_name)
            .send()
            .await?;
        Ok(())
    }

    async fn resume_group(&self, group_name: &str) -> Result<()> {
        info!(group = %group_name, "Resuming ASG processes");
        self.asg
            .resume_processes()
            .auto_scaling_group_name(group_name)
            .send()
            .await?;
        Ok(())
    }

    async fn stop_instance(&self, instance_id: &str) -> Result<()> {
        info!(instance = %instance_id, "Stopping ASG instance");
        self.ec2
            .stop_instances()
            .instance_ids(instance_id)
            .send()
            .await?;
        Ok(())
    }

    async fn start_instance(&self, instance_id: &str) -> Result<()> {
        info!(instance = %instance_id, "Starting ASG instance");
        self.ec2
            .start_instances()
            .instance_ids(instance_id)
            .send()
            .await?;
        Ok(())
    }

    /// Poll EC2 until all given instances are in the `running` state.
    async fn wait_instances_running(&self, instance_ids: &[String]) -> Result<()> {
        info!(count = instance_ids.len(), "Waiting for instances to reach running state");

        let max_attempts = 40;
        let delay = std::time::Duration::from_secs(15);

        for attempt in 1..=max_attempts {
            let mut req = self.ec2.describe_instance_status().include_all_instances(true);
            for id in instance_ids {
                req = req.instance_ids(id);
            }

            let resp = req.send().await?;

            let running_count = resp
                .instance_statuses()
                .iter()
                .filter(|s| {
                    s.instance_state()
                        .and_then(|st| st.name())
                        .map(|n| n.as_str() == "running")
                        .unwrap_or(false)
                })
                .count();

            if running_count == instance_ids.len() {
                info!("All instances are running");
                return Ok(());
            }

            info!(
                attempt,
                running = running_count,
                total = instance_ids.len(),
                "Waiting for instances..."
            );
            tokio::time::sleep(delay).await;
        }

        anyhow::bail!(
            "Timed out waiting for {} instances to reach running state",
            instance_ids.len()
        );
    }
}
