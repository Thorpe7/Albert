use crate::message_utils::ChatMessage;
use serde_json::to_string_pretty;
use std::fs::read_to_string;
use std::fs::File;
use std::io::Write;

pub struct ModelResponse {
    summaries: Summaries,
}
pub struct Summaries {
    author: String,
    summary: String,
}

pub fn write_messages_to_json(messages: &Vec<ChatMessage>) {
    let json_string = to_string_pretty(&messages).expect("Failed to serialize messages to JSON...");
    let mut output_file =
        File::create("chat_history.json").expect("Failed to create output file...");
    output_file
        .write_all(json_string.as_bytes())
        .expect("Failed to write to 'chat_history.json'...");
}

pub fn write_messages_to_txt(messages: &String) {
    let mut output_file =
        File::create("chat_history.txt").expect("Failed to create output file...");
    output_file
        .write_all(messages.as_bytes())
        .expect("Failed to write to 'chat_history.txt...");
}

pub fn read_json(file_path: Option<&str>) -> ModelResponse {
    let file_path = file_path.unwrap_or("model_response.json");
    let model_response = read_to_string(&file_path)?;




}
