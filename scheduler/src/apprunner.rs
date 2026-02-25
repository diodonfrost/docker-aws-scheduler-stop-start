use anyhow::Result;
use aws_sdk_apprunner::Client as AppRunnerClient;
use aws_sdk_resourcegroupstagging::Client as TaggingClient;
use tracing::{error, info};

use crate::filter_resources_by_tags;

/// Stop/start handler for AWS App Runner services in a given AWS region.
///
/// Uses the Resource Groups Tagging API to discover services matching a tag,
/// then pauses (stop) or resumes (start) each one.
pub struct AppRunnerScheduler {
    apprunner: AppRunnerClient,
    tagging: TaggingClient,
}

impl AppRunnerScheduler {
    pub async fn new(region: &str) -> Self {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        Self {
            apprunner: AppRunnerClient::new(&config),
            tagging: TaggingClient::new(&config),
        }
    }

    pub async fn stop(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns =
            filter_resources_by_tags::get_resources(&self.tagging, "apprunner:service", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found App Runner services to pause");

        for arn in &arns {
            let service_name = extract_service_name(arn);
            if let Err(e) = self.pause_service(arn).await {
                error!(service = %service_name, error = %e, "Failed to pause App Runner service");
            }
        }

        Ok(())
    }

    pub async fn start(&self, tag_key: &str, tag_value: &str) -> Result<()> {
        let arns =
            filter_resources_by_tags::get_resources(&self.tagging, "apprunner:service", tag_key, tag_value).await?;
        info!(count = arns.len(), "Found App Runner services to resume");

        for arn in &arns {
            let service_name = extract_service_name(arn);
            if let Err(e) = self.resume_service(arn).await {
                error!(service = %service_name, error = %e, "Failed to resume App Runner service");
            }
        }

        Ok(())
    }

    async fn pause_service(&self, service_arn: &str) -> Result<()> {
        let service_name = extract_service_name(service_arn);
        info!(service = %service_name, "Pausing App Runner service");
        self.apprunner
            .pause_service()
            .service_arn(service_arn)
            .send()
            .await?;
        Ok(())
    }

    async fn resume_service(&self, service_arn: &str) -> Result<()> {
        let service_name = extract_service_name(service_arn);
        info!(service = %service_name, "Resuming App Runner service");
        self.apprunner
            .resume_service()
            .service_arn(service_arn)
            .send()
            .await?;
        Ok(())
    }
}

/// Extract the service name from an App Runner service ARN.
///
/// Expected ARN format: `arn:aws:apprunner:region:account:service/name/id`
fn extract_service_name(arn: &str) -> String {
    let parts: Vec<&str> = arn.split('/').collect();
    if parts.len() >= 2 {
        parts[parts.len() - 2].to_string()
    } else {
        arn.to_string()
    }
}
