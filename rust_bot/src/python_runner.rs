use tokio::process::Command;


pub async fn run_python(file_id: &str) {
    if let Ok(status) = Command::new("python")
        .arg("python_llm/src/main.py")
        .arg(file_id)
        .status()
        .await
    {
        println!("{}", status);
        println!("Running python was successful!");
    }
}
