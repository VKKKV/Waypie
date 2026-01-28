use anyhow::{Context, Result};
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

    // Spawn the process. In Rust std::process::Command, children are detached
    // by default when the Child handle is dropped, unless `.wait()` is called.
    // However, to be fully independent (like double-fork), setsid is ideal but
    // strictly speaking simply spawning and not waiting is usually enough for
    // GUI launchers on Linux.
    Command::new(program)
        .args(arguments)
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn '{}': {}", program, e))?;

    Ok(())
}
