use serde_json;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;



pub fn write_messages_to_txt(messages: &String) {
    let mut output_file =
        File::create("chat_history.txt").expect("Failed to create output file...");
    output_file
        .write_all(messages.as_bytes())
        .expect("Failed to write to 'chat_history.txt...");
}

pub fn read_json(file_path: Option<&str>) -> Result<HashMap<String,String>, Box<dyn std::error::Error>> {
    let file_path = file_path.unwrap_or("model_response.json");
    let data = fs::read_to_string(&file_path)?;
    let model_response: HashMap<String,String> = serde_json::from_str(&data)?;
    println!("{:?}", model_response);
    Ok(model_response)
}
