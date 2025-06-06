mod bot;
mod message_utils;
mod python_runner;
mod read_and_write;

use bot::Handler;
use dotenv::dotenv;
use serenity::prelude::*;
use std::env;

// !NEXT STEPS: COULD NOT PUT TWO EMOTES AND GET MESSAGE FOR BOTH IN SEQUENCE
// TODO: Explicit download and install of local model & pre-load checkpoint shards in dockerfile
// TODO: Add logging and debugging logs for rust & python, esp to see model responses
// TODO: Add context window check mechanism
// TODO: Deploy, tbd where (EC2)

// !Testing Notes:
// TODO: Add timezone corrections so UTC timezone difference doesn't include yesterday's msgs
// TODO: Check that summarization works multiple times in a row and doesn't just respond with "No messages to summarize".

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

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client = Client::builder(&discord_token, intents)
        .event_handler(Handler)
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
