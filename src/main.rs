use std::env;

pub mod config;
pub mod hud;
pub mod tray;
pub mod utils;
pub mod sni_watcher;
pub mod color;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "daemon" {
        println!("Starting Waypie Daemon...");
        tray::run_daemon()?;
    } else {
        // Default to HUD
        let config = config::load();
        hud::run(config).map_err(|e| anyhow::anyhow!(e))?;
    }

    Ok(())
}