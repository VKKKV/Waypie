mod config;
mod color;
mod tray;
mod hud;
mod sni_watcher;
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

        // Create SNI watcher and items
        let sni_watcher = sni_watcher::SNIWatcher::new();
        let sni_items = sni_watcher.items();

        // Start SNI watcher in background on a separate thread
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Err(e) = sni_watcher.start().await {
                    if std::env::var("WAYPIE_DEBUG").is_ok() {
                        eprintln!("SNI Watcher error: {}", e);
                    }
                }
            });
        });

        // We run the tray logic as well so the icon appears in top bars if needed
        let _tray_handle = tray::run(config.clone()).await;
        hud::run("com.arch.waypie", config, sni_items);
    }
}
