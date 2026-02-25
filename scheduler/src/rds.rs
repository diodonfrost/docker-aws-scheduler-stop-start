use anyhow::Result;
use aws_sdk_rds::Client as RdsClient;
use aws_sdk_resourcegroupstagging::Client as TaggingClient;
use tracing::{error, info};

use crate::filter_resources_by_tags;

/// Stop/start handler for RDS instances and Aurora clusters in a given AWS region.
///
/// Uses the Resource Groups Tagging API to discover RDS clusters (`rds:cluster`)
/// and RDS instances (`rds:db`) matching a tag, then performs the requested action.
pub struct RdsScheduler {
    rds: RdsClient,
    tagging: TaggingClient,
}

impl RdsScheduler {
    pub async fn new(region: &str) -> Self {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        Self {
            rds: RdsClient::new(&config),
            tagging: TaggingClient::new(&config),
        }
    }

    pub async fn stop(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let cluster_arns =
            filter_resources_by_tags::get_resources(&self.tagging, "rds:cluster", tag_key, tag_value).await?;
        let instance_arns =
            filter_resources_by_tags::get_resources(&self.tagging, "rds:db", tag_key, tag_value).await?;

        info!(clusters = cluster_arns.len(), instances = instance_arns.len(), "Found RDS resources to stop");

        for arn in &cluster_arns {
            let cluster_id = extract_rds_id(arn);
            if let Err(e) = self.stop_cluster(&cluster_id).await {
                error!(cluster = %cluster_id, error = %e, "Failed to stop RDS cluster");
            }
        }

        for arn in &instance_arns {
            let db_id = extract_rds_id(arn);
            if let Err(e) = self.stop_instance(&db_id).await {
                error!(instance = %db_id, error = %e, "Failed to stop RDS instance");
            }
        }

        Ok(())
    }

    pub async fn start(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let cluster_arns =
            filter_resources_by_tags::get_resources(&self.tagging, "rds:cluster", tag_key, tag_value).await?;
        let instance_arns =
            filter_resources_by_tags::get_resources(&self.tagging, "rds:db", tag_key, tag_value).await?;

        info!(clusters = cluster_arns.len(), instances = instance_arns.len(), "Found RDS resources to start");

        for arn in &cluster_arns {
            let cluster_id = extract_rds_id(arn);
            if let Err(e) = self.start_cluster(&cluster_id).await {
                error!(cluster = %cluster_id, error = %e, "Failed to start RDS cluster");
            }
        }

        for arn in &instance_arns {
            let db_id = extract_rds_id(arn);
            if let Err(e) = self.start_instance(&db_id).await {
                error!(instance = %db_id, error = %e, "Failed to start RDS instance");
            }
        }

        Ok(())
    }

    async fn stop_cluster(&self, cluster_id: &str) -> Result<()> {
        info!(cluster = %cluster_id, "Stopping RDS cluster");
        self.rds
            .stop_db_cluster()
            .db_cluster_identifier(cluster_id)
            .send()
            .await?;
        Ok(())
    }

    async fn start_cluster(&self, cluster_id: &str) -> Result<()> {
        info!(cluster = %cluster_id, "Starting RDS cluster");
        self.rds
            .start_db_cluster()
            .db_cluster_identifier(cluster_id)
            .send()
            .await?;
        Ok(())
    }

    async fn stop_instance(&self, db_id: &str) -> Result<()> {
        info!(instance = %db_id, "Stopping RDS instance");
        self.rds
            .stop_db_instance()
            .db_instance_identifier(db_id)
            .send()
            .await?;
        Ok(())
    }

    async fn start_instance(&self, db_id: &str) -> Result<()> {
        info!(instance = %db_id, "Starting RDS instance");
        self.rds
            .start_db_instance()
            .db_instance_identifier(db_id)
            .send()
            .await?;
        Ok(())
    }
}

/// Extract the resource identifier from an RDS ARN.
///
/// Expected ARN formats:
/// - Cluster: `arn:aws:rds:region:account:cluster:cluster-id`
/// - Instance: `arn:aws:rds:region:account:db:instance-id`
fn extract_rds_id(arn: &str) -> String {
    arn.split(':').last().unwrap_or(arn).to_string()
}
