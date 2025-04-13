use crate::read_and_write::Summaries;
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

pub fn format_json_to_message(json_data: &Summaries) -> String {
    json_data
        .summaries
        .iter()
        .map(|s| format!("**{}**: {}", s.author, s.summary))
        .collect::<Vec<_>>()
        .join("\n")
}
