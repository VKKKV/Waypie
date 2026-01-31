use crate::color::{ColorRGB, ColorRGBA};
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

/// Convert hex color (0xRRGGBB) to RGB tuple with normalized values (0.0-1.0)
pub const fn hex_to_rgb(hex: u32) -> ColorRGB {
    let r = ((hex >> 16) & 0xFF) as f64 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f64 / 255.0;
    let b = (hex & 0xFF) as f64 / 255.0;
    (r, g, b)
}

/// Convert hex color (0xRRGGBBAA) to RGBA tuple with normalized values (0.0-1.0)
pub const fn hex_to_rgba(hex: u32) -> ColorRGBA {
    let r = ((hex >> 24) & 0xFF) as f64 / 255.0;
    let g = ((hex >> 16) & 0xFF) as f64 / 255.0;
    let b = ((hex >> 8) & 0xFF) as f64 / 255.0;
    let a = (hex & 0xFF) as f64 / 255.0;
    (r, g, b, a)
}
