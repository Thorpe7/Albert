use crate::message_utils::{ChatMessage,string_format_today_messages};
use serde::Deserialize;
use serde_json;
use serde_json::to_string_pretty;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct Summary {
    pub summary: String,
    pub content: String,
}

pub fn write_messages_to_json(messages: &Vec<ChatMessage>) {
    let json_string = to_string_pretty(&messages).expect("Failed to serialize messages to JSON...");
    let mut output_file =
        File::create("chat_history.json").expect("Failed to create output file...");
    output_file
        .write_all(json_string.as_bytes())
        .expect("Failed to write to 'chat_history.json'...");
}

pub fn write_messages_to_txt(messages_today: &Vec<HashMap<String, String>>) -> Result<String,std::io::Error>{

    let formatted_messages: String = string_format_today_messages(&messages_today); 
    let id = Uuid::new_v4().to_string();
    let filepath = format!("/tmp/input_{id}.txt");
    let mut output_file = File::create(&filepath).expect("Failed to create output file...");
    output_file.write_all(formatted_messages.as_bytes())?;
    Ok(filepath.to_string())
}

pub fn read_json(file_path: Option<&str>) -> Result<HashMap<String,String>, Box<dyn std::error::Error>> {
    let file_path = file_path.unwrap_or("model_response.json");
    let data = fs::read_to_string(&file_path)?;
    let model_response: HashMap<String,String> = serde_json::from_str(&data)?;
    println!("{:?}", model_response);
    Ok(model_response)
}
