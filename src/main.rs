use dotenv::dotenv;
use serde::Serialize;
use serde_json::to_string_pretty;
use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::builder::GetMessages;
use serenity::model::channel::Message;
use serenity::model::channel::Reaction;
use serenity::model::gateway::Ready;
use serenity::model::prelude::ReactionType;
use serenity::prelude::*;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use time::{Date, OffsetDateTime, Time};

// !NEXT STEPS:
// TODO: Format & clean up code to align w/ rust standards better
// TODO: Addition of python code LangChain + Local optimized LLM
// TODO: Connect components
// TODO: Containerize
// TODO: Deploy, tbd where.
#[derive(Serialize)]
struct ChatMessage {
    author: String,
    content: String,
}

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
                    let now = OffsetDateTime::now_utc();
                    let today =
                        Date::from_calendar_date(now.year(), now.month(), now.day()).unwrap();
                    let start_of_today = today.with_time(Time::MIDNIGHT).assume_utc();
                    let mut messages_today: Vec<HashMap<String, String>> = Vec::new();

                    let message_getter = GetMessages::new().limit(100);
                    let result_history = reaction
                        .channel_id
                        .messages(&ctx.http, message_getter)
                        .await;
                    if let Ok(history) = result_history {
                        for chat in history.iter() {
                            if chat.timestamp.to_utc() > start_of_today {
                                let mut entry = HashMap::new();
                                entry.insert(chat.author.name.clone(), chat.content.clone());
                                messages_today.push(entry)
                            }
                        }

                        let json_formatted_messages: Vec<ChatMessage> = history
                            .iter()
                            .rev()
                            .filter(|msg| msg.timestamp.to_utc() > start_of_today)
                            .map(|msg| ChatMessage {
                                author: msg.author.name.clone(),
                                content: msg.content.clone(),
                            })
                            .collect();
                        let json_string = to_string_pretty(&json_formatted_messages)
                            .expect("Failed to serialize messages to JSON...");
                        let mut output_file = File::create("chat_history.json")
                            .expect("Failed to create output file...");
                        output_file
                            .write_all(json_string.as_bytes())
                            .expect("Failed to write to 'chat_history.json'...");
                    }

                    let formatted_messages: String = messages_today
                        .iter()
                        .rev()
                        .flat_map(|entry| entry.iter())
                        .map(|(username, content)| format!("**{}**: {}", username, content))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let dm = CreateMessage::new().content(&formatted_messages);

                    if let Err(why) = msg.author.direct_message(&ctx.http, dm).await {
                        println!("Failed to send dm to user: {why:?}")
                    }
                }
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
