mod apprunner;
mod autoscaling;
mod cloudwatch;
mod config;
mod documentdb;
mod ec2;
mod ecs;
mod filter_resources_by_tags;
mod rds;
mod redshift;
mod transfer;

use anyhow::Result;
use chrono::Utc;
use tracing::{error, info};

use config::{AppConfig, ScheduleAction};

/// Application entry point.
///
/// Loads configuration from environment variables, then performs
/// the stop/start action on AWS resources matching the configured tag.
#[tokio::main]
async fn main() -> Result<()> {
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(&log_level))
        .init();

    let config = AppConfig::from_env()?;

    info!(
        action = %config.schedule_action,
        regions = ?config.aws_regions,
        tag = %format!("{}={}", config.tag_key, config.tag_value),
        ec2 = config.ec2_schedule,
        apprunner = config.apprunner_schedule,
        autoscaling = config.autoscaling_schedule,
        cloudwatch = config.cloudwatch_alarm_schedule,
        documentdb = config.documentdb_schedule,
        ecs = config.ecs_schedule,
        rds = config.rds_schedule,
        redshift = config.redshift_schedule,
        transfer = config.transfer_schedule,
        "Scheduler initialized"
    );

    execute(&config).await
}

/// Check whether today's date (`MM-DD` format) is in the exclusion list.
fn is_date_excluded(excluded_dates: &[String]) -> bool {
    let today = Utc::now().format("%m-%d").to_string();
    excluded_dates.iter().any(|d| d == &today)
}

/// Execute the stop/start action across all configured regions.
///
/// Skips execution if today is an excluded date.
/// Errors on individual regions are logged without interrupting the processing of others.
async fn execute(config: &AppConfig) -> Result<()> {
    if is_date_excluded(&config.excluded_dates) {
        info!(
            date = %Utc::now().format("%m-%d"),
            "Today is an excluded date, skipping execution"
        );
        return Ok(());
    }

    for region in &config.aws_regions {
        if config.ec2_schedule {
            info!(region = %region, action = %config.schedule_action, "Processing EC2 instances");
            let scheduler = ec2::Ec2Scheduler::new(region).await;
            let result = match config.schedule_action {
                ScheduleAction::Stop => scheduler.stop(&config.tag_key, &config.tag_value).await,
                ScheduleAction::Start => scheduler.start(&config.tag_key, &config.tag_value).await,
            };
            if let Err(e) = result {
                error!(region = %region, error = %e, "Failed to process EC2 instances");
            }
        }

        if config.autoscaling_schedule {
            info!(region = %region, action = %config.schedule_action, "Processing Auto Scaling groups");
            let scheduler = autoscaling::AutoScalingScheduler::new(region).await;
            let result = match config.schedule_action {
                ScheduleAction::Stop => scheduler.stop(&config.tag_key, &config.tag_value).await,
                ScheduleAction::Start => scheduler.start(&config.tag_key, &config.tag_value).await,
            };
            if let Err(e) = result {
                error!(region = %region, error = %e, "Failed to process Auto Scaling groups");
            }
        }

        if config.apprunner_schedule {
            info!(region = %region, action = %config.schedule_action, "Processing App Runner services");
            let scheduler = apprunner::AppRunnerScheduler::new(region).await;
            let result = match config.schedule_action {
                ScheduleAction::Stop => scheduler.stop(&config.tag_key, &config.tag_value).await,
                ScheduleAction::Start => scheduler.start(&config.tag_key, &config.tag_value).await,
            };
            if let Err(e) = result {
                error!(region = %region, error = %e, "Failed to process App Runner services");
            }
        }

        if config.cloudwatch_alarm_schedule {
            info!(region = %region, action = %config.schedule_action, "Processing CloudWatch alarms");
            let scheduler = cloudwatch::CloudWatchScheduler::new(region).await;
            let result = match config.schedule_action {
                ScheduleAction::Stop => scheduler.stop(&config.tag_key, &config.tag_value).await,
                ScheduleAction::Start => scheduler.start(&config.tag_key, &config.tag_value).await,
            };
            if let Err(e) = result {
                error!(region = %region, error = %e, "Failed to process CloudWatch alarms");
            }
        }

        if config.documentdb_schedule {
            info!(region = %region, action = %config.schedule_action, "Processing DocumentDB clusters");
            let scheduler = documentdb::DocumentDbScheduler::new(region).await;
            let result = match config.schedule_action {
                ScheduleAction::Stop => scheduler.stop(&config.tag_key, &config.tag_value).await,
                ScheduleAction::Start => scheduler.start(&config.tag_key, &config.tag_value).await,
            };
            if let Err(e) = result {
                error!(region = %region, error = %e, "Failed to process DocumentDB clusters");
            }
        }

        if config.ecs_schedule {
            info!(region = %region, action = %config.schedule_action, "Processing ECS services");
            let scheduler = ecs::EcsScheduler::new(region).await;
            let result = match config.schedule_action {
                ScheduleAction::Stop => scheduler.stop(&config.tag_key, &config.tag_value).await,
                ScheduleAction::Start => scheduler.start(&config.tag_key, &config.tag_value).await,
            };
            if let Err(e) = result {
                error!(region = %region, error = %e, "Failed to process ECS services");
            }
        }

        if config.rds_schedule {
            info!(region = %region, action = %config.schedule_action, "Processing RDS resources");
            let scheduler = rds::RdsScheduler::new(region).await;
            let result = match config.schedule_action {
                ScheduleAction::Stop => scheduler.stop(&config.tag_key, &config.tag_value).await,
                ScheduleAction::Start => scheduler.start(&config.tag_key, &config.tag_value).await,
            };
            if let Err(e) = result {
                error!(region = %region, error = %e, "Failed to process RDS resources");
            }
        }

        if config.redshift_schedule {
            info!(region = %region, action = %config.schedule_action, "Processing Redshift clusters");
            let scheduler = redshift::RedshiftScheduler::new(region).await;
            let result = match config.schedule_action {
                ScheduleAction::Stop => scheduler.stop(&config.tag_key, &config.tag_value).await,
                ScheduleAction::Start => scheduler.start(&config.tag_key, &config.tag_value).await,
            };
            if let Err(e) = result {
                error!(region = %region, error = %e, "Failed to process Redshift clusters");
            }
        }

        if config.transfer_schedule {
            info!(region = %region, action = %config.schedule_action, "Processing Transfer servers");
            let scheduler = transfer::TransferScheduler::new(region).await;
            let result = match config.schedule_action {
                ScheduleAction::Stop => scheduler.stop(&config.tag_key, &config.tag_value).await,
                ScheduleAction::Start => scheduler.start(&config.tag_key, &config.tag_value).await,
            };
            if let Err(e) = result {
                error!(region = %region, error = %e, "Failed to process Transfer servers");
            }
        }
    }

    info!("Execution completed");
    Ok(())
}
