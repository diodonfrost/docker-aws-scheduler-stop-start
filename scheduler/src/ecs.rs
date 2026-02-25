use anyhow::Result;
use aws_sdk_ecs::Client as EcsClient;
use aws_sdk_resourcegroupstagging::Client as TaggingClient;
use tracing::{error, info};

use crate::filter_resources_by_tags;

/// Stop/start handler for ECS services in a given AWS region.
///
/// Uses the Resource Groups Tagging API to discover services matching a tag,
/// then sets the desired count to 0 (stop) or 1 (start).
pub struct EcsScheduler {
    ecs: EcsClient,
    tagging: TaggingClient,
}

impl EcsScheduler {
    pub async fn new(region: &str) -> Self {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        Self {
            ecs: EcsClient::new(&config),
            tagging: TaggingClient::new(&config),
        }
    }

    pub async fn stop(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns =
            filter_resources_by_tags::get_resources(&self.tagging, "ecs:service", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found ECS services to stop");

        for arn in &arns {
            let (cluster, service) = extract_ecs_names(arn);
            if let Err(e) = self.update_service(&cluster, &service, 0).await {
                error!(service = %service, cluster = %cluster, error = %e, "Failed to stop ECS service");
            }
        }

        Ok(())
    }

    pub async fn start(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns =
            filter_resources_by_tags::get_resources(&self.tagging, "ecs:service", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found ECS services to start");

        for arn in &arns {
            let (cluster, service) = extract_ecs_names(arn);
            if let Err(e) = self.update_service(&cluster, &service, 1).await {
                error!(service = %service, cluster = %cluster, error = %e, "Failed to start ECS service");
            }
        }

        Ok(())
    }

    async fn update_service(&self, cluster: &str, service: &str, desired_count: i32) -> Result<()> {
        let action = if desired_count == 0 { "Stopping" } else { "Starting" };
        info!(service = %service, cluster = %cluster, desired_count, "{action} ECS service");
        self.ecs
            .update_service()
            .cluster(cluster)
            .service(service)
            .desired_count(desired_count)
            .send()
            .await?;
        Ok(())
    }
}

/// Extract the cluster name and service name from an ECS service ARN.
///
/// Expected ARN format: `arn:aws:ecs:region:account:service/cluster-name/service-name`
fn extract_ecs_names(arn: &str) -> (String, String) {
    let parts: Vec<&str> = arn.split('/').collect();
    if parts.len() >= 3 {
        (parts[parts.len() - 2].to_string(), parts[parts.len() - 1].to_string())
    } else {
        (String::new(), arn.to_string())
    }
}
