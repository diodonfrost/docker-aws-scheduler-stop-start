variable "region" {
  description = "AWS region where the test resources will be created."
  type        = string
  default     = "eu-west-1"
}

variable "tag_key" {
  description = "Tag key used by the scheduler to discover resources."
  type        = string
  default     = "scheduler-e2e"
}

variable "tag_value" {
  description = "Tag value used by the scheduler to discover resources."
  type        = string
  default     = "true"
}
