use serenity::all::{Channel, Message};
use std::collections::HashMap;
use serenity::builder::CreateMessage;
use serenity::builder::GetMessages;
use serenity::client::Context;
use serenity::model::channel::Reaction;

use crate::message_utils::{
    format_json_to_message, get_start_of_today, string_format_today_messages,
};
use crate::python_runner::run_python;
use crate::read_and_write::{read_json, write_messages_to_txt};


pub async fn summarize_chat(msg:Message, ctx: &Context, reaction: Reaction) {       
        let mut channel_name = String::new();
        if let Ok(channel) = msg.channel_id.to_channel(&ctx.http).await {
            if let Channel::Guild(guild_channel) = channel {
                println!("Channel name: {}", guild_channel.name);
                channel_name = guild_channel.name;
            } else {
                println!("Not a guild channel.");
            }
        } else {
                println!("Failed to fetch channel.");
        }

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
            let model_response = match read_json(None) {
                Ok(data) => data,
                Err(e) => {
                    println!("Failed to read JSON: {e}");
                    return;
                }
            };
            let message_to_user = format_json_to_message(&model_response,&channel_name);
            dm = CreateMessage::new().content(&message_to_user);
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
