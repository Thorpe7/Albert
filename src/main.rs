use dotenv::dotenv;
use std::env;

use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::model::channel::Message;
use serenity::model::channel::Reaction;
use serenity::model::gateway::Ready;
use serenity::model::prelude::ReactionType;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // Handler struct for message event - called when new message is received.

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message: {why:?}");
            }
        }
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        if let ReactionType::Unicode(ref emoji) = reaction.emoji {
            if emoji == "🤖" {
                if let Ok(msg) = reaction
                    .channel_id
                    .message(&ctx.http, reaction.message_id)
                    .await
                {
                    let dm = CreateMessage::new().content("Detected robot emoji use...");

                    if let Err(why) = msg.author.direct_message(&ctx.http, dm).await {
                        println!("Failed to send dm to user: {why:?}")
                    }
                }
            }

            if let Err(why) = reaction
                .channel_id
                .say(&ctx.http, "Detected robot emoji use...")
                .await
            {
                println!("Error sending message about emoji: {why:?}");
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name)
    }
}

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
