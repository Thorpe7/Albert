mod article_handler;
mod bedrock_client;
mod bot_functions;
mod handle_events;
mod message_utils;
mod response_target;
mod worker_and_job;

use handle_events::Handler;
use bedrock_client::BedrockClient;
use dotenv::dotenv;
use serenity::prelude::*;
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::worker_and_job::{start_worker, Job};

#[tokio::main]
async fn main() {
    dotenv().ok();
    let discord_token: String =
        env::var("DISCORD_TOKEN").expect("Expected 'DISCORD_TOKEN' in environment...");

    let bedrock_client = Arc::new(
        BedrockClient::new().await.expect("Failed to initialize BedrockClient")
    );

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let (tx, rx) = mpsc::channel::<Job>(32);
    let handler = Handler { tx };
    start_worker(rx, bedrock_client);

    let mut client = Client::builder(&discord_token, intents)
        .event_handler(handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
