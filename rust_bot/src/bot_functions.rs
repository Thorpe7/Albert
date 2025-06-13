use serenity::all::{Channel, Message};
use std::collections::HashMap;
use serenity::builder::CreateMessage;
use serenity::builder::GetMessages;
use serenity::client::Context;
use serenity::model::channel::Reaction;
use uuid::Uuid;
use anyhow::Result;

use crate::message_utils::{
    format_json_to_message, get_start_of_today, string_format_today_messages,
};
use crate::python_runner::run_python;
use crate::read_and_write::{read_json, write_messages_to_txt};


pub async fn summarize_chat(file_id:Uuid, msg:Message, ctx: &Context, reaction: Reaction) -> Result<()>{       
        let mut channel_name = String::new();
        let response_path = format!("{}/model_response.json",file_id);
        if let Ok(channel) = msg.channel_id.to_channel(&ctx.http).await {
            if let Channel::Guild(guild_channel) = channel {
                channel_name = guild_channel.name;
            } else {
                return Err(anyhow::anyhow!("({}) is not a guild channel.", channel_name));
            }
        } else {
                return Err(anyhow::anyhow!("Failed to fetch channel."));
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
                    // println!("{}", chat.timestamp.to_utc());
                    // println!("{}", start_of_today);
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
            let msg_hx_path = write_messages_to_txt(&formatted_messages, &file_id);
            if let Err(e) = run_python(&msg_hx_path).await {
                return Err(anyhow::anyhow!("Running python script failed: {}",e));
            }
            let model_response = match read_json(&response_path) {
                Ok(data) => data,
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to read JSON ({}): {}",&response_path,e));
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
                    return Err(anyhow::anyhow!("Failed to send dm to user: {why:?}"));
                }
            } else {
                return Err(anyhow::anyhow!("Failed to fetch user from user_id..."));
            }
        } else {
            return Err(anyhow::anyhow!("No user_id on reaction..."));
        }
    Ok(())
}