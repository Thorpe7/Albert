use serde::Serialize;
use std::collections::HashMap;
use time::{Date, OffsetDateTime, Time, UtcOffset};

#[derive(Serialize)]
pub struct ChatMessage {
    pub author: String,
    pub content: String,
}

pub fn get_start_of_today() -> time::OffsetDateTime {
    let pt_offset = UtcOffset::from_hms(-8, 0, 0).unwrap();
    let now = OffsetDateTime::now_utc().to_offset(pt_offset);
    let today = Date::from_calendar_date(now.year(), now.month(), now.day()).unwrap();
    today.with_time(Time::MIDNIGHT).assume_offset(pt_offset)
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
