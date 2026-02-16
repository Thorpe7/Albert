use serenity::all::Channel;
use serenity::model::id::ChannelId;
use std::collections::HashMap;
use time::OffsetDateTime;
use serenity::client::Context;
use anyhow::Result;
use serenity::builder::GetMessages;


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

pub async fn get_channel_name(channel_id: ChannelId, ctx: &Context) -> Result<String> {
    let channel = channel_id.to_channel(&ctx.http).await
        .map_err(|_| anyhow::anyhow!("Failed to fetch channel."))?;
    match channel {
        Channel::Guild(guild_channel) => Ok(guild_channel.name),
        _ => Err(anyhow::anyhow!("Not a guild channel.")),
    }
}

pub async fn get_messages(channel_id: ChannelId, ctx: &Context) -> Vec<HashMap<String,String>> {
    let cutoff = get_24h_ago();
    let mut messages_today: Vec<HashMap<String, String>> = Vec::new();
    let message_getter = GetMessages::new().limit(100);
    let result_history = channel_id
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
