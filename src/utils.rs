use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::path::PathBuf;
use std::process::Command;

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

#[cfg(test)]
mod tests {
    use super::cartesian_to_polar;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn cartesian_to_polar_right_of_center() {
        let (dist, angle) = cartesian_to_polar(2.0, 1.0, 1.0, 1.0);
        assert!(approx_eq(dist, 1.0));
        assert!(approx_eq(angle, 0.0));
    }

    #[test]
    fn cartesian_to_polar_above_center() {
        let (dist, angle) = cartesian_to_polar(1.0, 0.0, 1.0, 1.0);
        assert!(approx_eq(dist, 1.0));
        assert!(approx_eq(angle, -90.0));
    }

    #[test]
    fn cartesian_to_polar_same_point_as_center() {
        let (dist, angle) = cartesian_to_polar(1.0, 1.0, 1.0, 1.0);
        assert!(approx_eq(dist, 0.0));
        assert!(approx_eq(angle, 0.0));
    }
}
