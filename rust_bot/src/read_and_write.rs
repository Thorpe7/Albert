use crate::message_utils::ChatMessage;
use serde::Deserialize;
use serde_json;
use serde_json::to_string_pretty;
use std::fs;
use std::fs::File;
use std::io::Write;

#[derive(Debug, Deserialize)]
pub struct Summary {
    pub author: String,
    pub summary: String,
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

pub fn read_json(file_path: Option<&str>) -> Result<Vec<Summary>, Box<dyn std::error::Error>> {
    let file_path = file_path.unwrap_or("model_response.json");
    let data = fs::read_to_string(&file_path)?;
    let model_response: Vec<Summary> = serde_json::from_str(&data)?;
    println!("{:?}", model_response);
    Ok(model_response)
}
