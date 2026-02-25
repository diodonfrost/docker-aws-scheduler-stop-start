use anyhow::Result;
use aws_sdk_docdb::Client as DocDbClient;
use aws_sdk_resourcegroupstagging::Client as TaggingClient;
use tracing::{error, info};

use crate::filter_resources_by_tags;

/// Stop/start handler for DocumentDB clusters in a given AWS region.
///
/// Uses the Resource Groups Tagging API to discover clusters matching a tag,
/// then performs the requested action on each one.
pub struct DocumentDbScheduler {
    docdb: DocDbClient,
    tagging: TaggingClient,
}

impl DocumentDbScheduler {
    pub async fn new(region: &str) -> Self {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        Self {
            docdb: DocDbClient::new(&config),
            tagging: TaggingClient::new(&config),
        }
    }

    pub async fn stop(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns =
            filter_resources_by_tags::get_resources(&self.tagging, "rds:cluster", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found DocumentDB clusters to stop");

        for arn in &arns {
            let cluster_id = extract_cluster_id(arn);
            if let Err(e) = self.stop_cluster(&cluster_id).await {
                error!(cluster = %cluster_id, error = %e, "Failed to stop DocumentDB cluster");
            }
        }

        Ok(())
    }

    pub async fn start(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns =
            filter_resources_by_tags::get_resources(&self.tagging, "rds:cluster", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found DocumentDB clusters to start");

        for arn in &arns {
            let cluster_id = extract_cluster_id(arn);
            if let Err(e) = self.start_cluster(&cluster_id).await {
                error!(cluster = %cluster_id, error = %e, "Failed to start DocumentDB cluster");
            }
        }

        Ok(())
    }

    async fn stop_cluster(&self, cluster_id: &str) -> Result<()> {
        info!(cluster = %cluster_id, "Stopping DocumentDB cluster");
        self.docdb
            .stop_db_cluster()
            .db_cluster_identifier(cluster_id)
            .send()
            .await?;
        Ok(())
    }

    async fn start_cluster(&self, cluster_id: &str) -> Result<()> {
        info!(cluster = %cluster_id, "Starting DocumentDB cluster");
        self.docdb
            .start_db_cluster()
            .db_cluster_identifier(cluster_id)
            .send()
            .await?;
        Ok(())
    }
}

/// Extract the cluster identifier from an RDS cluster ARN.
///
/// Expected ARN format: `arn:aws:rds:region:account:cluster:cluster-id`
fn extract_cluster_id(arn: &str) -> String {
    arn.split(':').last().unwrap_or(arn).to_string()
}
