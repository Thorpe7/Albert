use serde_json;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use uuid::Uuid;
use std::path::Path;


pub fn write_messages_to_txt(messages: &String, file_id: &Uuid ) -> String {
    let dir_path_string = format!("jobs/{}", file_id.hyphenated());
    let dir_path = Path::new(&dir_path_string);
    if !dir_path.exists() {
        fs::create_dir_all(dir_path).unwrap();
    }
    let msg_hx_path = format!("{}/chat_history.txt", dir_path_string);
    let mut output_file =
        File::create(&msg_hx_path).expect("Failed to create output file...");
    output_file
        .write_all(messages.as_bytes())
        .expect("Failed to write to 'chat_history.txt...");

    msg_hx_path
}

pub fn write_article_to_txt(article_text: &str, file_id: &Uuid) -> String {
    let dir_path_string = format!("jobs/{}", file_id.hyphenated());
    let dir_path = Path::new(&dir_path_string);
    if !dir_path.exists() {
        fs::create_dir_all(dir_path).unwrap();
    }
    let article_path = format!("{}/article_content.txt", dir_path_string);
    let mut output_file =
        File::create(&article_path).expect("Failed to create output file...");
    output_file
        .write_all(article_text.as_bytes())
        .expect("Failed to write to 'article_content.txt'...");

    article_path
}

pub fn read_json(file_path: &str) -> Result<HashMap<String,String>, Box<dyn std::error::Error>> {
    // let file_path = file_path.to_string();
    let data = fs::read_to_string(&file_path)?;
    let model_response: HashMap<String,String> = serde_json::from_str(&data)?;
    // println!("{:?}", model_response);
    Ok(model_response)
}
