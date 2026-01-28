use std::process::Command;

pub fn execute_command(cmd: &str) {
    let status = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .spawn();
    
    if let Err(e) = status {
        eprintln!("Failed to execute command '{}': {}", cmd, e);
    }
}
