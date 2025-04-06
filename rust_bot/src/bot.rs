use crate::export::write_messages_to_txt;
use crate::message_utils::{get_start_of_today, string_format_today_messages};
use crate::python_runner::run_python;
use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::builder::GetMessages;
use serenity::model::channel::Message;
use serenity::model::channel::Reaction;
use serenity::model::gateway::Ready;
use serenity::model::prelude::ReactionType;
use serenity::prelude::*;
use std::collections::HashMap;
pub struct Handler;

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
                    let start_of_today = get_start_of_today();
                    let mut messages_today: Vec<HashMap<String, String>> = Vec::new();
                    let message_getter = GetMessages::new().limit(100);
                    let result_history = reaction
                        .channel_id
                        .messages(&ctx.http, message_getter)
                        .await;

                    if let Ok(history) = result_history {
                        for chat in history.iter() {
                            if chat.timestamp.to_utc() >= start_of_today {
                                println!("{}", chat.timestamp.to_utc());
                                println!("{}", start_of_today);
                                let mut entry = HashMap::new();
                                entry.insert(chat.author.name.clone(), chat.content.clone());
                                messages_today.push(entry)
                            }
                        }
                    }
                    let dm: CreateMessage;
                    if messages_today.len() > 1 {
                        let formatted_messages: String =
                            string_format_today_messages(&messages_today);
                        write_messages_to_txt(&formatted_messages);
                        run_python();
                        dm = CreateMessage::new().content(&formatted_messages);
                    } else {
                        dm = CreateMessage::new().content("No messages found to summarize...");
                    }

                    if let Some(user_id) = reaction.user_id {
                        if let Ok(user) = user_id.to_user(&ctx.http).await {
                            if let Err(why) = user.direct_message(&ctx.http, dm).await {
                                println!("Failed to send dm to user: {why:?}")
                            }
                        } else {
                            println!("Failed to fetch user from user_id...")
                        }
                    } else {
                        println!("No user_id on reaction...");
                    }
                }
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name)
    }
}
