use anyhow::Result;
use aws_sdk_cloudwatch::Client as CloudWatchClient;
use aws_sdk_resourcegroupstagging::Client as TaggingClient;
use tracing::{error, info};

use crate::filter_resources_by_tags;

/// Stop/start handler for CloudWatch alarm actions in a given AWS region.
///
/// Uses the Resource Groups Tagging API to discover alarms matching a tag,
/// then enables or disables alarm actions on each one.
pub struct CloudWatchScheduler {
    cloudwatch: CloudWatchClient,
    tagging: TaggingClient,
}

impl CloudWatchScheduler {
    pub async fn new(region: &str) -> Self {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        Self {
            cloudwatch: CloudWatchClient::new(&config),
            tagging: TaggingClient::new(&config),
        }
    }

    pub async fn stop(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns =
            filter_resources_by_tags::get_resources(&self.tagging, "cloudwatch:alarm", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found CloudWatch alarms to disable");

        for arn in &arns {
            let alarm_name = extract_alarm_name(arn);
            if let Err(e) = self.disable_alarm(&alarm_name).await {
                error!(alarm = %alarm_name, error = %e, "Failed to disable alarm");
            }
        }

        Ok(())
    }

    pub async fn start(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns =
            filter_resources_by_tags::get_resources(&self.tagging, "cloudwatch:alarm", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found CloudWatch alarms to enable");

        for arn in &arns {
            let alarm_name = extract_alarm_name(arn);
            if let Err(e) = self.enable_alarm(&alarm_name).await {
                error!(alarm = %alarm_name, error = %e, "Failed to enable alarm");
            }
        }

        Ok(())
    }

    async fn disable_alarm(&self, alarm_name: &str) -> Result<()> {
        info!(alarm = %alarm_name, "Disabling alarm actions");
        self.cloudwatch
            .disable_alarm_actions()
            .alarm_names(alarm_name)
            .send()
            .await?;
        Ok(())
    }

    async fn enable_alarm(&self, alarm_name: &str) -> Result<()> {
        info!(alarm = %alarm_name, "Enabling alarm actions");
        self.cloudwatch
            .enable_alarm_actions()
            .alarm_names(alarm_name)
            .send()
            .await?;
        Ok(())
    }
}

/// Extract the alarm name from a CloudWatch alarm ARN.
///
/// Expected ARN format: `arn:aws:cloudwatch:region:account:alarm:name`
fn extract_alarm_name(arn: &str) -> String {
    arn.split(':').last().unwrap_or(arn).to_string()
}
