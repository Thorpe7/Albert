use crate::export::write_messages_to_json;
use crate::message_utils::{get_start_of_today, string_format_today_messages, ChatMessage};
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
                        write_messages_to_json(&json_formatted_messages);
                    }

                    let formatted_messages: String = string_format_today_messages(&messages_today);
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
