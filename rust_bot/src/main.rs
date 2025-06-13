mod handle_events;
mod message_utils;
mod python_runner;
mod read_and_write;
mod worker_and_job;
mod bot_functions;

use handle_events::Handler;
use dotenv::dotenv;
use serenity::prelude::*;
use std::env;
use tokio::sync::mpsc;
use crate::worker_and_job::{start_worker, Job};

// !NEXT STEPS:
// TODO: Add logging and debugging logs for rust & python, esp to see model responses
// TODO: Add context window check mechanism
// TODO: Cache recent summaries w/ last message datetime check
// TODO: Summary broken down by user
// TODO: Summarize article

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    dotenv().ok();
    let discord_token: String =
        env::var("DISCORD_TOKEN").expect("Expected 'DISCORD_TOKEN' in environment...");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let (tx, rx) = mpsc::channel::<Job>(32);
    let handler = Handler{tx};
    start_worker(rx);

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client = Client::builder(&discord_token, intents)
        .event_handler(handler)
        .await
        .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until
    // it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
