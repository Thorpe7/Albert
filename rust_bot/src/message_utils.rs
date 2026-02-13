use serde::Serialize;
use serenity::all::{Channel,Message, Reaction};
use uuid::Uuid;
use std::collections::HashMap;
use time::OffsetDateTime;
use serenity::client::Context;
use anyhow::Result;
use serenity::builder::GetMessages;

#[derive(Serialize)]
pub struct ChatMessage {
    pub author: String,
    pub content: String,
}

pub fn get_24h_ago() -> time::OffsetDateTime {
    OffsetDateTime::now_utc() - time::Duration::hours(24)
}

pub fn string_format_today_messages(messages_today: &Vec<HashMap<String, String>>) -> String {
    messages_today
        .iter()
        .rev()
        .flat_map(|entry| entry.iter())
        .map(|(username, content)| format!("Author: **{}**; Content: {}", username, content))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_json_to_message(json_data: &HashMap<String,String>, channel_name: &String) -> String {
    let message_str = match json_data.get("summary") {
        Some(val) => val,
        None => {
            println!("Summary not found...");
            return String::from("No summary available...");
        }
    };

    format!("**Channel: **{}\n{}", channel_name, message_str)
}

pub async fn get_channel_name(file_id: Uuid, msg: Message, ctx: &Context) -> Result<(String, String)> {
    let mut channel_name = String::new();
    let response_path = format!("jobs/{}/model_response.json",file_id);
    if let Ok(channel) = msg.channel_id.to_channel(&ctx.http).await {
        if let Channel::Guild(guild_channel) = channel {
            channel_name = guild_channel.name;
        } else {
            return Err(anyhow::anyhow!("({}) is not a guild channel.", channel_name));
        }
    } else {
            return Err(anyhow::anyhow!("Failed to fetch channel."));
    }
    Ok((channel_name, response_path))
}

pub async fn get_messages(reaction: &Reaction, ctx: &Context) -> Vec<HashMap<String,String>> {
    let cutoff = get_24h_ago();
    let mut messages_today: Vec<HashMap<String, String>> = Vec::new();
    let message_getter = GetMessages::new().limit(100);
    let result_history = reaction
        .channel_id
        .messages(&ctx.http, message_getter)
        .await;

    if let Ok(history) = result_history {
        for chat in history.iter() {
            if chat.timestamp.to_utc() >= cutoff {
                // println!("{}", chat.timestamp.to_utc());
                // println!("{}", start_of_today);
                let mut entry = HashMap::new();
                entry.insert(chat.author.name.clone(), chat.content.clone());
                messages_today.push(entry)
            }
        }
    }
    messages_today
}
