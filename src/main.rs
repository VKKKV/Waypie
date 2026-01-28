mod config;
mod tray;
mod hud;
mod utils;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    // Usage:
    // waypie       -> Radial Wheel (Center HUD + Ring Tray)
    // waypie daemon -> Background Tray Service

    if args.len() > 1 && args[1] == "daemon" {
        // Background Service
        let config = config::load();
        let _tray_handle = tray::run(config).await;
        // Keep running
        std::future::pending::<()>().await;
    } else {
        // Default: Radial Wheel (Hud + Tray)
        let config = config::load();
        // We run the tray logic as well so the icon appears in top bars if needed
        let _tray_handle = tray::run(config.clone()).await;
        hud::run("com.arch.waypie", config);
    }
}