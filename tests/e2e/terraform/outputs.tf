output "instance_id" {
  description = "ID of the EC2 instance created for e2e testing."
  value       = aws_instance.e2e_test.id
}

output "region" {
  description = "AWS region where the test instance was created."
  value       = var.region
}

output "tag_key" {
  description = "Tag key used for resource discovery."
  value       = var.tag_key
}

output "tag_value" {
  description = "Tag value used for resource discovery."
  value       = var.tag_value
}
