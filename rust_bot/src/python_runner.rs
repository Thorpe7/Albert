use serde::Deserialize;
use std::fs;
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct Summary {
    author: String,
    summary: String,
}

#[derive(Debug, Deserialize)]
pub struct ModelResponse {
    summaries: Vec<Summary>,
}

pub fn run_python(filepath: &String) {
    if let Ok(status) = Command::new("python")
        .arg("python_llm/src/main.py")
        .arg(filepath)
        .status()
    {
        println!("{}", status);
        println!("Running python was successful!");
    }
}
