use anyhow::{Context, Result};
use std::process::Command;
use std::path::PathBuf;
use directories::ProjectDirs;

/// Spawns an application in a detached way.
/// Handles shell argument parsing (e.g. quotes) using `shlex`.
pub fn spawn_app(command_str: &str) -> Result<()> {
    let args = shlex::split(command_str)
        .context("Failed to parse command arguments (unbalanced quotes?)")?;

    if args.is_empty() {
        return Ok(());
    }

    let program = &args[0];
    let arguments = &args[1..];

    Command::new(program)
        .args(arguments)
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn '{}': {}", program, e))?;

    Ok(())
}

/// Converts Cartesian coordinates (x, y) to Polar coordinates (radius, angle_degrees).
/// Center is (cx, cy).
pub fn cartesian_to_polar(x: f64, y: f64, cx: f64, cy: f64) -> (f64, f64) {
    let dx = x - cx;
    let dy = y - cy;
    let dist = (dx * dx + dy * dy).sqrt();
    let theta_rad = dy.atan2(dx);
    let theta_deg = theta_rad.to_degrees();
    (dist, theta_deg)
}

/// Returns the path to the configuration file.
pub fn get_config_path() -> Option<PathBuf> {
    ProjectDirs::from("org", "waypie", "waypie").map(|proj| proj.config_dir().join("config.toml"))
}
