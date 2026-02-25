use anyhow::Result;
use aws_sdk_transfer::Client as TransferClient;
use aws_sdk_resourcegroupstagging::Client as TaggingClient;
use tracing::{error, info};

use crate::filter_resources_by_tags;

/// Stop/start handler for AWS Transfer Family servers in a given AWS region.
///
/// Uses the Resource Groups Tagging API to discover servers matching a tag,
/// then performs the requested action on each one.
pub struct TransferScheduler {
    transfer: TransferClient,
    tagging: TaggingClient,
}

impl TransferScheduler {
    pub async fn new(region: &str) -> Self {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        Self {
            transfer: TransferClient::new(&config),
            tagging: TaggingClient::new(&config),
        }
    }

    pub async fn stop(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns =
            filter_resources_by_tags::get_resources(&self.tagging, "transfer:server", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found Transfer servers to stop");

        for arn in &arns {
            let server_id = extract_server_id(arn);
            if let Err(e) = self.stop_server(&server_id).await {
                error!(server = %server_id, error = %e, "Failed to stop Transfer server");
            }
        }

        Ok(())
    }

    pub async fn start(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns =
            filter_resources_by_tags::get_resources(&self.tagging, "transfer:server", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found Transfer servers to start");

        for arn in &arns {
            let server_id = extract_server_id(arn);
            if let Err(e) = self.start_server(&server_id).await {
                error!(server = %server_id, error = %e, "Failed to start Transfer server");
            }
        }

        Ok(())
    }

    async fn stop_server(&self, server_id: &str) -> Result<()> {
        info!(server = %server_id, "Stopping Transfer server");
        self.transfer
            .stop_server()
            .server_id(server_id)
            .send()
            .await?;
        Ok(())
    }

    async fn start_server(&self, server_id: &str) -> Result<()> {
        info!(server = %server_id, "Starting Transfer server");
        self.transfer
            .start_server()
            .server_id(server_id)
            .send()
            .await?;
        Ok(())
    }
}

/// Extract the server ID from a Transfer Family server ARN.
///
/// Expected ARN format: `arn:aws:transfer:region:account:server/server-id`
fn extract_server_id(arn: &str) -> String {
    arn.split('/').last().unwrap_or(arn).to_string()
}
