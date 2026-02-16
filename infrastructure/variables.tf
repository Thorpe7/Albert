variable "aws_region" {
  description = "AWS region for all resources"
  type        = string
  default     = "us-east-1"
}

variable "discord_public_key" {
  description = "Discord application public key (for signature verification in Lambda A)"
  type        = string
  sensitive   = true
}

variable "discord_bot_token" {
  description = "Discord bot token (for API calls in Lambda B)"
  type        = string
  sensitive   = true
}

variable "discord_application_id" {
  description = "Discord application ID (for interaction callbacks in Lambda B)"
  type        = string
}

variable "gateway_zip_path" {
  description = "Path to Lambda A bootstrap.zip (built by cargo lambda)"
  type        = string
  default     = "../target/lambda/lambda_gateway/bootstrap.zip"
}

variable "worker_zip_path" {
  description = "Path to Lambda B bootstrap.zip (built by cargo lambda)"
  type        = string
  default     = "../target/lambda/lambda_worker/bootstrap.zip"
}
