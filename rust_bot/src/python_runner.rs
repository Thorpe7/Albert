use std::process::Command;


pub fn run_python() {
    if let Ok(status) = Command::new("python")
        .arg("python_llm/src/main.py")
        .status()
    {
        println!("{}", status);
        println!("Running python was successful!");
    }
}
