use anyhow::Result;
use aws_sdk_redshift::Client as RedshiftClient;
use aws_sdk_resourcegroupstagging::Client as TaggingClient;
use tracing::{error, info};

use crate::filter_resources_by_tags;

/// Stop/start handler for Redshift clusters in a given AWS region.
///
/// Uses the Resource Groups Tagging API to discover clusters matching a tag,
/// then pauses (stop) or resumes (start) each one.
pub struct RedshiftScheduler {
    redshift: RedshiftClient,
    tagging: TaggingClient,
}

impl RedshiftScheduler {
    pub async fn new(region: &str) -> Self {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        Self {
            redshift: RedshiftClient::new(&config),
            tagging: TaggingClient::new(&config),
        }
    }

    pub async fn stop(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns =
            filter_resources_by_tags::get_resources(&self.tagging, "redshift:cluster", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found Redshift clusters to pause");

        for arn in &arns {
            let cluster_id = extract_cluster_id(arn);
            if let Err(e) = self.pause_cluster(&cluster_id).await {
                error!(cluster = %cluster_id, error = %e, "Failed to pause Redshift cluster");
            }
        }

        Ok(())
    }

    pub async fn start(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns =
            filter_resources_by_tags::get_resources(&self.tagging, "redshift:cluster", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found Redshift clusters to resume");

        for arn in &arns {
            let cluster_id = extract_cluster_id(arn);
            if let Err(e) = self.resume_cluster(&cluster_id).await {
                error!(cluster = %cluster_id, error = %e, "Failed to resume Redshift cluster");
            }
        }

        Ok(())
    }

    async fn pause_cluster(&self, cluster_id: &str) -> Result<()> {
        info!(cluster = %cluster_id, "Pausing Redshift cluster");
        self.redshift
            .pause_cluster()
            .cluster_identifier(cluster_id)
            .send()
            .await?;
        Ok(())
    }

    async fn resume_cluster(&self, cluster_id: &str) -> Result<()> {
        info!(cluster = %cluster_id, "Resuming Redshift cluster");
        self.redshift
            .resume_cluster()
            .cluster_identifier(cluster_id)
            .send()
            .await?;
        Ok(())
    }
}

/// Extract the cluster identifier from a Redshift cluster ARN.
///
/// Expected ARN format: `arn:aws:redshift:region:account:cluster:cluster-id`
fn extract_cluster_id(arn: &str) -> String {
    arn.split(':').last().unwrap_or(arn).to_string()
}
