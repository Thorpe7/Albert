use tokio::process::Command;


pub async fn run_python(file_id: &str) -> std::io::Result<()> {
    let status = Command::new("python")
        .arg("python_llm/src/main.py")
        .arg(file_id)
        .status()
        .await?;
    if status.success() {
        println!("{}", status);
        Ok(())
    } else {
        println!("Running python was successful!");
        Err(std::io::Error::new(std::io::ErrorKind::Other, "Python script failed..."))
    }
}
