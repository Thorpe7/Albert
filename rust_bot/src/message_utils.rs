use serde::Serialize;
use std::collections::HashMap;
use time::{ Date, OffsetDateTime, Time, UtcOffset};
use std::io::Error;
use serenity::model::channel::Message;
use serenity::client::Context;
use serenity::all::Channel;
use serenity::builder::{GetMessages, CreateMessage};
use serenity::model::channel::Reaction;
use serenity::prelude::SerenityError;

#[derive(Serialize)]
pub struct ChatMessage {
    pub author: String,
    pub content: String,
}

pub fn get_start_of_today() -> Result<time::OffsetDateTime, time::error::ComponentRange>  {
    let local_offset = UtcOffset::from_hms(-6, 0, 0)?;
    let now = OffsetDateTime::now_utc().to_offset(local_offset);
    let today = Date::from_calendar_date(now.year(), now.month(), now.day()).unwrap();
    Ok(today.with_time(Time::MIDNIGHT).assume_utc())
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
pub async fn get_channel_name(msg: Message, chnl_context: &Context) -> Result<String, Error> {
    let mut channel_name = String::new();
    if let Ok(channel) = msg.channel_id.to_channel(&chnl_context.http).await {
        if let Channel::Guild(guild_channel) = channel {
            println!("Channel name: {}", guild_channel.name);
            channel_name = guild_channel.name;
        } else {
            println!("Not a guild channel.");
        }
    } else {
            println!("Failed to fetch channel.");
    }
    Ok(channel_name)
}

pub async fn get_today_channel_hx(reaction: &Reaction, chnl_context: &Context) -> (OffsetDateTime,Result<Vec<Message>,SerenityError>) {
    let start_of_today = get_start_of_today().unwrap(); // NEEDS ERROR HANDLING
    let message_getter = GetMessages::new().limit(100); // CURRENT MSG HX LIMIT CAP 100 MSGS
    let history_result = reaction
        .channel_id
        .messages(&chnl_context.http, message_getter)
        .await;
    (start_of_today, history_result)
}

pub async fn send_dm_to_user(reaction: &Reaction, chnl_context: &Context, dm: CreateMessage) -> Result<(),SerenityError>{
    let user_id = reaction.user_id.unwrap(); // Determine if panic handling needed
    let user = user_id.to_user(&chnl_context.http).await.unwrap(); // Determine if panic handling needed
    match user.direct_message(&chnl_context.http, dm).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e)
    }

}
