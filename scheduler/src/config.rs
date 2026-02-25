use anyhow::{bail, Context, Result};
use std::env;

/// Read a boolean from an environment variable (case-insensitive "true"/"false").
/// Returns `default` when the variable is not set.
fn env_bool(name: &str, default: bool) -> bool {
    env::var(name)
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(default)
}

/// Action to perform on AWS resources.
#[derive(Debug, Clone)]
pub enum ScheduleAction {
    /// Stop the resources.
    Stop,
    /// Start the resources.
    Start,
}

impl std::fmt::Display for ScheduleAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleAction::Stop => write!(f, "stop"),
            ScheduleAction::Start => write!(f, "start"),
        }
    }
}

/// Application configuration loaded from environment variables.
///
/// Required variables:
/// - `SCHEDULE_ACTION`: `stop` or `start`
/// - `AWS_REGIONS`: comma-separated list of AWS regions
/// - `TAG_KEY`: tag key to filter resources
/// - `TAG_VALUE`: tag value to filter resources
///
/// Optional variables (each defaults to `false` unless noted):
/// - `EC2_SCHEDULE`: enable EC2 processing (default: `true`)
/// - `APPRUNNER_SCHEDULE`: enable App Runner processing
/// - `AUTOSCALING_SCHEDULE`: enable Auto Scaling Group processing
/// - `CLOUDWATCH_ALARM_SCHEDULE`: enable CloudWatch alarm processing
/// - `DOCUMENTDB_SCHEDULE`: enable DocumentDB processing
/// - `ECS_SCHEDULE`: enable ECS service processing
/// - `RDS_SCHEDULE`: enable RDS instance/cluster processing
/// - `REDSHIFT_SCHEDULE`: enable Redshift cluster processing
/// - `TRANSFER_SCHEDULE`: enable Transfer Family server processing
/// - `EXCLUDED_DATES`: comma-separated dates in `MM-DD` format to skip execution
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub schedule_action: ScheduleAction,
    pub aws_regions: Vec<String>,
    pub tag_key: String,
    pub tag_value: String,
    pub ec2_schedule: bool,
    pub apprunner_schedule: bool,
    pub autoscaling_schedule: bool,
    pub cloudwatch_alarm_schedule: bool,
    pub documentdb_schedule: bool,
    pub ecs_schedule: bool,
    pub rds_schedule: bool,
    pub redshift_schedule: bool,
    pub transfer_schedule: bool,
    pub excluded_dates: Vec<String>,
}

impl AppConfig {
    /// Load configuration from environment variables.
    ///
    /// Returns an error if required variables are missing or invalid.
    pub fn from_env() -> Result<Self> {
        let schedule_action = match env::var("SCHEDULE_ACTION")
            .context("SCHEDULE_ACTION env var is required (stop|start)")?
            .to_lowercase()
            .as_str()
        {
            "stop" => ScheduleAction::Stop,
            "start" => ScheduleAction::Start,
            other => bail!(
                "Invalid SCHEDULE_ACTION '{}': must be 'stop' or 'start'",
                other
            ),
        };

        let aws_regions: Vec<String> = env::var("AWS_REGIONS")
            .context("AWS_REGIONS env var is required (comma-separated)")?
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if aws_regions.is_empty() {
            bail!("AWS_REGIONS must contain at least one region");
        }

        let tag_key = env::var("TAG_KEY").context("TAG_KEY env var is required")?;
        let tag_value = env::var("TAG_VALUE").context("TAG_VALUE env var is required")?;

        let ec2_schedule = env_bool("EC2_SCHEDULE", true);
        let apprunner_schedule = env_bool("APPRUNNER_SCHEDULE", false);
        let autoscaling_schedule = env_bool("AUTOSCALING_SCHEDULE", false);
        let cloudwatch_alarm_schedule = env_bool("CLOUDWATCH_ALARM_SCHEDULE", false);
        let documentdb_schedule = env_bool("DOCUMENTDB_SCHEDULE", false);
        let ecs_schedule = env_bool("ECS_SCHEDULE", false);
        let rds_schedule = env_bool("RDS_SCHEDULE", false);
        let redshift_schedule = env_bool("REDSHIFT_SCHEDULE", false);
        let transfer_schedule = env_bool("TRANSFER_SCHEDULE", false);

        let excluded_dates: Vec<String> = env::var("EXCLUDED_DATES")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(Self {
            schedule_action,
            aws_regions,
            tag_key,
            tag_value,
            ec2_schedule,
            apprunner_schedule,
            autoscaling_schedule,
            cloudwatch_alarm_schedule,
            documentdb_schedule,
            ecs_schedule,
            rds_schedule,
            redshift_schedule,
            transfer_schedule,
            excluded_dates,
        })
    }
}
