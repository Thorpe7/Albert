use crate::read_and_write::Summary;
use serde::Serialize;
use std::collections::HashMap;
use time::{Date, OffsetDateTime, Time};

#[derive(Serialize)]
pub struct ChatMessage {
    pub author: String,
    pub content: String,
}

pub fn get_start_of_today() -> time::OffsetDateTime {
    let now = OffsetDateTime::now_utc();
    let today = Date::from_calendar_date(now.year(), now.month(), now.day()).unwrap();
    today.with_time(Time::MIDNIGHT).assume_utc()
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

pub fn format_json_to_message(json_data: &Vec<Summary>, channel_name: &String) -> String {
    let message_str = json_data
        .iter()
        .map(|s| format!("**{}**:\n\t{}", s.author, s.summary))
        .collect::<Vec<_>>()
        .join("\n");
    format!("**Channel: **{}\n{}", channel_name, message_str)
}
