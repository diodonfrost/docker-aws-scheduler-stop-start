## docker-aws-scheduler-stop-start

[![Build](https://github.com/diodonfrost/docker-aws-scheduler-stop-start/actions/workflows/build.yml/badge.svg)](https://github.com/diodonfrost/docker-aws-scheduler-stop-start/actions/workflows/build.yml)

Docker-based AWS resource scheduler that stops and starts AWS resources based on tags. Designed to be invoked by external schedulers (Lambda, CronJob, ECS Scheduled Task, Rundeck) to reduce cloud costs by powering down non-production resources during off-hours.

### Supported AWS Services

- EC2 instances (excludes instances managed by Auto Scaling Groups)
- Auto Scaling Groups
- RDS instances and Aurora clusters
- ECS services
- App Runner services
- CloudWatch Alarms
- DocumentDB clusters
- Redshift clusters
- Transfer Family servers

## How to Build

This image is built on Github registry automatically any time a commit is made or merged to the `master` branch. But if you need to build the image on your own locally, do the following:

  1. [Install Docker](https://docs.docker.com/engine/installation/).
  2. `cd` into this directory.
  3. Run `docker build -t aws-scheduler-stop-start .`

## How to Use

```bash
docker run \
  -e SCHEDULE_ACTION=stop \
  -e AWS_REGIONS=eu-west-1 \
  -e TAG_KEY=env \
  -e TAG_VALUE=staging \
  -e EC2_SCHEDULE=true \
  -e RDS_SCHEDULE=true \
  aws-scheduler-stop-start
```

AWS credentials must be available in the container (via environment variables, instance profile, or mounted `~/.aws` directory).

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `SCHEDULE_ACTION` | Yes | | `stop` or `start` |
| `AWS_REGIONS` | Yes | | Comma-separated list of AWS regions (e.g. `eu-west-1,us-east-1`) |
| `TAG_KEY` | Yes | | Tag key used to filter resources |
| `TAG_VALUE` | Yes | | Tag value used to filter resources |
| `EC2_SCHEDULE` | No | `true` | Enable EC2 instance scheduling |
| `AUTOSCALING_SCHEDULE` | No | `false` | Enable Auto Scaling Group scheduling |
| `RDS_SCHEDULE` | No | `false` | Enable RDS instance and Aurora cluster scheduling |
| `ECS_SCHEDULE` | No | `false` | Enable ECS service scheduling |
| `APPRUNNER_SCHEDULE` | No | `false` | Enable App Runner service scheduling |
| `CLOUDWATCH_ALARM_SCHEDULE` | No | `false` | Enable CloudWatch Alarm scheduling |
| `DOCUMENTDB_SCHEDULE` | No | `false` | Enable DocumentDB cluster scheduling |
| `REDSHIFT_SCHEDULE` | No | `false` | Enable Redshift cluster scheduling |
| `TRANSFER_SCHEDULE` | No | `false` | Enable Transfer Family server scheduling |
| `EXCLUDED_DATES` | No | | Comma-separated dates in `MM-DD` format to skip execution (e.g. `12-25,01-01`) |
| `LOG_LEVEL` | No | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |

## Authors

Modules managed by [diodonfrost](https://github.com/diodonfrost)

## Licence

Apache 2 Licensed. See LICENSE for full details.
