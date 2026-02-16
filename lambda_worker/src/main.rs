mod article_handler;
mod bedrock_client;
mod discord_client;
mod handlers;

use bedrock_client::BedrockClient;
use discord_client::DiscordClient;
use handlers::Interaction;
use lambda_runtime::{service_fn, LambdaEvent};
use serde_json::Value;
use std::env;

async fn handler(event: LambdaEvent<Value>) -> Result<Value, lambda_runtime::Error> {
    let (payload, _context) = event.into_parts();

    let interaction: Interaction = serde_json::from_value(payload)
        .map_err(|e| lambda_runtime::Error::from(format!("Failed to parse interaction: {}", e)))?;

    let command_name = interaction.data.as_ref()
        .and_then(|d| d.name.as_deref())
        .unwrap_or("unknown");

    tracing::info!(command = command_name, "Processing command");

    let bot_token = env::var("DISCORD_BOT_TOKEN")
        .map_err(|_| lambda_runtime::Error::from("DISCORD_BOT_TOKEN env var required"))?;
    let app_id = env::var("DISCORD_APPLICATION_ID")
        .map_err(|_| lambda_runtime::Error::from("DISCORD_APPLICATION_ID env var required"))?;

    let bedrock = BedrockClient::new().await
        .map_err(|e| lambda_runtime::Error::from(format!("Failed to init Bedrock: {}", e)))?;
    let discord = DiscordClient::new(bot_token, app_id);

    let result = match command_name {
        "summary-24hr" => {
            handlers::handle_summary_chat(&interaction, "STANDARD_SUMMARY", &discord, &bedrock).await
        }
        "summary-peruser" => {
            handlers::handle_summary_chat(&interaction, "PER_USER_SUMMARY", &discord, &bedrock).await
        }
        "summary-article" => {
            handlers::handle_summary_article_slash(&interaction, &discord, &bedrock).await
        }
        "Summarize Article" => {
            handlers::handle_summary_article_context_menu(&interaction, &discord, &bedrock).await
        }
        _ => {
            tracing::warn!(command = command_name, "Unknown command");
            Ok(())
        }
    };

    if let Err(e) = result {
        tracing::error!(error = %e, command = command_name, "Command failed");
        // Always try to edit the deferred response so users don't see infinite "thinking..."
        let error_msg = format!("Something went wrong: {}", e);
        if let Err(edit_err) = discord.edit_original_response(&interaction.token, &error_msg).await {
            tracing::error!(error = %edit_err, "Failed to send error response to user");
        }
    }

    Ok(serde_json::json!({"status": "ok"}))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    lambda_runtime::run(service_fn(handler)).await
}
