output "api_gateway_url" {
  description = "Set this as the Interactions Endpoint URL in Discord Developer Portal"
  value       = "${aws_apigatewayv2_stage.default.invoke_url}discord-interactions"
}

output "gateway_function_name" {
  description = "Lambda A function name"
  value       = aws_lambda_function.gateway.function_name
}

output "worker_function_name" {
  description = "Lambda B function name"
  value       = aws_lambda_function.worker.function_name
}
