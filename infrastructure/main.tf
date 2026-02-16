terraform {
  required_version = ">= 1.5"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

data "aws_caller_identity" "current" {}

# ---------------------------------------------------------------------------
# IAM — Lambda A (Gateway)
# ---------------------------------------------------------------------------

resource "aws_iam_role" "gateway" {
  name = "albert-gateway-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = { Service = "lambda.amazonaws.com" }
    }]
  })
}

resource "aws_iam_role_policy_attachment" "gateway_logs" {
  role       = aws_iam_role.gateway.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

resource "aws_iam_role_policy" "gateway_invoke_worker" {
  name = "invoke-worker"
  role = aws_iam_role.gateway.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action   = "lambda:InvokeFunction"
      Effect   = "Allow"
      Resource = aws_lambda_function.worker.arn
    }]
  })
}

# ---------------------------------------------------------------------------
# IAM — Lambda B (Worker)
# ---------------------------------------------------------------------------

resource "aws_iam_role" "worker" {
  name = "albert-worker-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = { Service = "lambda.amazonaws.com" }
    }]
  })
}

resource "aws_iam_role_policy_attachment" "worker_logs" {
  role       = aws_iam_role.worker.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

resource "aws_iam_role_policy" "worker_bedrock" {
  name = "bedrock-invoke"
  role = aws_iam_role.worker.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action   = "bedrock:InvokeModel"
      Effect   = "Allow"
      Resource = [
        "arn:aws:bedrock:*::foundation-model/anthropic.claude-3-5-haiku-20241022-v1:0",
        "arn:aws:bedrock:*:${data.aws_caller_identity.current.account_id}:inference-profile/us.anthropic.claude-3-5-haiku-20241022-v1:0"
      ]
    }]
  })
}

resource "aws_iam_role_policy" "worker_dynamodb" {
  name = "dynamodb-crud"
  role = aws_iam_role.worker.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = [
        "dynamodb:GetItem",
        "dynamodb:PutItem",
        "dynamodb:UpdateItem",
        "dynamodb:DeleteItem",
        "dynamodb:Query",
        "dynamodb:Scan"
      ]
      Effect   = "Allow"
      Resource = "arn:aws:dynamodb:${var.aws_region}:${data.aws_caller_identity.current.account_id}:table/albert-*"
    }]
  })
}

# ---------------------------------------------------------------------------
# Lambda Functions
# ---------------------------------------------------------------------------

resource "aws_lambda_function" "gateway" {
  function_name = "albert-gateway"
  role          = aws_iam_role.gateway.arn
  handler       = "bootstrap"
  runtime       = "provided.al2023"
  architectures = ["x86_64"]
  timeout       = 5
  memory_size   = 128
  filename      = var.gateway_zip_path
  source_code_hash = filebase64sha256(var.gateway_zip_path)

  environment {
    variables = {
      DISCORD_PUBLIC_KEY   = var.discord_public_key
      WORKER_FUNCTION_NAME = aws_lambda_function.worker.function_name
      RUST_LOG             = "info"
    }
  }

  depends_on = [
    aws_iam_role_policy_attachment.gateway_logs,
    aws_cloudwatch_log_group.gateway,
  ]
}

resource "aws_lambda_function" "worker" {
  function_name = "albert-worker"
  role          = aws_iam_role.worker.arn
  handler       = "bootstrap"
  runtime       = "provided.al2023"
  architectures = ["x86_64"]
  timeout       = 60
  memory_size   = 256
  filename      = var.worker_zip_path
  source_code_hash = filebase64sha256(var.worker_zip_path)

  environment {
    variables = {
      DISCORD_BOT_TOKEN      = var.discord_bot_token
      DISCORD_APPLICATION_ID = var.discord_application_id
      RUST_LOG               = "info"
    }
  }

  depends_on = [
    aws_iam_role_policy_attachment.worker_logs,
    aws_cloudwatch_log_group.worker,
  ]
}

# ---------------------------------------------------------------------------
# API Gateway (HTTP API)
# ---------------------------------------------------------------------------

resource "aws_apigatewayv2_api" "discord" {
  name          = "albert-discord-api"
  protocol_type = "HTTP"
}

resource "aws_apigatewayv2_integration" "gateway_lambda" {
  api_id                 = aws_apigatewayv2_api.discord.id
  integration_type       = "AWS_PROXY"
  integration_uri        = aws_lambda_function.gateway.invoke_arn
  payload_format_version = "2.0"
}

resource "aws_apigatewayv2_route" "discord_interactions" {
  api_id    = aws_apigatewayv2_api.discord.id
  route_key = "POST /discord-interactions"
  target    = "integrations/${aws_apigatewayv2_integration.gateway_lambda.id}"
}

resource "aws_apigatewayv2_stage" "default" {
  api_id      = aws_apigatewayv2_api.discord.id
  name        = "$default"
  auto_deploy = true
}

resource "aws_lambda_permission" "apigw_invoke_gateway" {
  statement_id  = "AllowAPIGatewayInvoke"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.gateway.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.discord.execution_arn}/*/*"
}

# ---------------------------------------------------------------------------
# DynamoDB Tables
# ---------------------------------------------------------------------------

resource "aws_dynamodb_table" "articles" {
  name         = "albert-articles"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "url_hash"

  attribute {
    name = "url_hash"
    type = "S"
  }

  ttl {
    attribute_name = "ttl"
    enabled        = true
  }
}

resource "aws_dynamodb_table" "summaries" {
  name         = "albert-summaries"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "message_id"

  attribute {
    name = "message_id"
    type = "S"
  }

  ttl {
    attribute_name = "ttl"
    enabled        = true
  }
}

resource "aws_dynamodb_table" "dm_sessions" {
  name         = "albert-dm-sessions"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "user_id"
  range_key    = "session_id"

  attribute {
    name = "user_id"
    type = "S"
  }

  attribute {
    name = "session_id"
    type = "S"
  }

  ttl {
    attribute_name = "ttl"
    enabled        = true
  }
}

# ---------------------------------------------------------------------------
# CloudWatch Log Groups
# ---------------------------------------------------------------------------

resource "aws_cloudwatch_log_group" "gateway" {
  name              = "/aws/lambda/albert-gateway"
  retention_in_days = 14
}

resource "aws_cloudwatch_log_group" "worker" {
  name              = "/aws/lambda/albert-worker"
  retention_in_days = 14
}
