use anyhow::Result;
use aws_sdk_autoscaling::Client as AsgClient;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_resourcegroupstagging::Client as TaggingClient;
use tracing::{error, info};

use crate::filter_resources_by_tags;

/// Stop/start handler for EC2 instances in a given AWS region.
///
/// Uses the Resource Groups Tagging API to discover instances matching a tag,
/// then performs the requested action on each one.
/// Instances belonging to an Auto Scaling Group are automatically skipped.
pub struct Ec2Scheduler {
    ec2: Ec2Client,
    asg: AsgClient,
    tagging: TaggingClient,
}

/// Action to perform on an individual EC2 instance.
enum Action {
    Stop,
    Start,
}

impl Ec2Scheduler {
    /// Create a new EC2 scheduler for the given region.
    ///
    /// Initializes AWS clients (EC2, Auto Scaling, Resource Groups Tagging)
    /// with credentials resolved automatically by the SDK.
    pub async fn new(region: &str) -> Self {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        Self {
            ec2: Ec2Client::new(&config),
            asg: AsgClient::new(&config),
            tagging: TaggingClient::new(&config),
        }
    }

    /// Stop all EC2 instances matching the given tag.
    ///
    /// Instances belonging to an Auto Scaling Group are skipped.
    /// Errors on individual instances are logged without interrupting the processing.
    pub async fn stop(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns: Vec<String> =
            filter_resources_by_tags::get_resources(&self.tagging, "ec2:instance", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found EC2 instances to stop");

        for arn in &arns {
            let instance_id = extract_instance_id(arn);
            if let Err(e) = self.process_instance(&instance_id, Action::Stop).await {
                error!(instance_id = %instance_id, error = %e, "Failed to stop instance");
            }
        }

        Ok(())
    }

    /// Start all EC2 instances matching the given tag.
    ///
    /// Instances belonging to an Auto Scaling Group are skipped.
    /// Errors on individual instances are logged without interrupting the processing.
    pub async fn start(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns: Vec<String> =
            filter_resources_by_tags::get_resources(&self.tagging, "ec2:instance", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found EC2 instances to start");

        for arn in &arns {
            let instance_id = extract_instance_id(arn);
            if let Err(e) = self.process_instance(&instance_id, Action::Start).await {
                error!(instance_id = %instance_id, error = %e, "Failed to start instance");
            }
        }

        Ok(())
    }

    /// Process a single EC2 instance.
    ///
    /// Checks whether the instance belongs to an Auto Scaling Group first.
    /// If so, the instance is skipped. Otherwise, the stop/start action is performed.
    async fn process_instance(&self, instance_id: &str, action: Action) -> Result<()> {
        let asg_response = self
            .asg
            .describe_auto_scaling_instances()
            .instance_ids(instance_id)
            .send()
            .await?;

        if !asg_response.auto_scaling_instances().is_empty() {
            info!(
                instance_id = %instance_id,
                "Skipping instance (belongs to Auto Scaling Group)"
            );
            return Ok(());
        }

        match action {
            Action::Stop => {
                info!(instance_id = %instance_id, "Stopping instance");
                self.ec2
                    .stop_instances()
                    .instance_ids(instance_id)
                    .send()
                    .await?;
            }
            Action::Start => {
                info!(instance_id = %instance_id, "Starting instance");
                self.ec2
                    .start_instances()
                    .instance_ids(instance_id)
                    .send()
                    .await?;
            }
        }

        Ok(())
    }
}

/// Extract the instance ID from an EC2 ARN.
///
/// Expected ARN format: `arn:aws:ec2:region:account:instance/i-xxxxx`
fn extract_instance_id(arn: &str) -> String {
    arn.split('/').last().unwrap_or(arn).to_string()
}
