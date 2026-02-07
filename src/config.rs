use directories::ProjectDirs;
use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

// 1. Data Structures
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default = "default_menu_items")]
    pub menu: Vec<MenuItemConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ui: UiConfig::default(),
            menu: default_menu_items(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UiConfig {
    #[serde(default = "default_width")]
    pub width: i32,
    #[serde(default = "default_height")]
    pub height: i32,
    #[serde(default = "default_center_radius")]
    pub center_radius: f64,
    #[serde(default = "default_inner_radius")]
    pub inner_radius: f64,
    #[serde(default = "default_outer_radius")]
    pub outer_radius: f64,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
            center_radius: default_center_radius(),
            inner_radius: default_inner_radius(),
            outer_radius: default_outer_radius(),
        }
    }
}

fn default_width() -> i32 { 600 }
fn default_height() -> i32 { 600 }
fn default_center_radius() -> f64 { 40.0 }
fn default_inner_radius() -> f64 { 100.0 }
fn default_outer_radius() -> f64 { 200.0 }

fn default_menu_items() -> Vec<MenuItemConfig> {
    vec![
        MenuItemConfig {
            label: "Web".to_string(),
            icon: "web-browser".to_string(),
            action: "".to_string(),
            children: vec![
                MenuItemConfig {
                    label: "Firefox".to_string(),
                    icon: "firefox".to_string(),
                    action: "firefox".to_string(),
                    children: vec![],
                    item_type: None,
                },
                MenuItemConfig {
                    label: "zen-browser".to_string(),
                    icon: "zen-browser".to_string(),
                    action: "zen-browser".to_string(),
                    children: vec![],
                    item_type: None,
                },
            ],
            item_type: None,
        },
        MenuItemConfig {
            label: "Terminal".to_string(),
            icon: "utilities-terminal".to_string(),
            action: "ghostty".to_string(),
            children: vec![],
            item_type: None,
        },
        MenuItemConfig {
            label: "Files".to_string(),
            icon: "system-file-manager".to_string(),
            action: "thunar".to_string(),
            children: vec![],
            item_type: None,
        },
        MenuItemConfig {
            label: "Tray".to_string(),
            icon: "emblem-system".to_string(), // Generic icon
            action: "".to_string(),
            children: vec![],
            item_type: Some("tray".to_string()),
        },
    ]
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MenuItemConfig {
    pub label: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub children: Vec<MenuItemConfig>,
    #[serde(default, rename = "type")]
    pub item_type: Option<String>,
}

// 2. Loading Logic
pub fn load_config() -> Config {
    let path = get_config_path();
    if let Some(p) = &path {
        if p.exists() {
            match fs::read_to_string(p) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(cfg) => return cfg,
                    Err(e) => {
                        eprintln!("Error parsing config at {:?}: {}", p, e);
                        eprintln!("Falling back to default config.");
                    }
                },
                Err(e) => eprintln!("Error reading config file: {}", e),
            }
        } else {
            println!("Config not found. Creating default at {:?}", p);
            if let Some(parent) = p.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let default_cfg = Config::default();
            if let Ok(toml_string) = toml::to_string_pretty(&default_cfg) {
                if let Err(e) = fs::write(p, toml_string) {
                    eprintln!("Failed to write default config: {}", e);
                }
            }
            return default_cfg;
        }
    }
    Config::default()
}

fn get_config_path() -> Option<PathBuf> {
    ProjectDirs::from("org", "waypie", "waypie").map(|proj| proj.config_dir().join("config.toml"))
}

// 3. Watcher Setup
pub async fn watch_config(config_store: Arc<RwLock<Config>>, sender: async_channel::Sender<()>) {
    let path = get_config_path().unwrap_or_else(|| PathBuf::from("config.toml"));
    let (tx, mut rx) = mpsc::channel(1);

    // Create a watcher that sends events to the channel
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.blocking_send(res);
        },
        NotifyConfig::default(),
    )
    .expect("Failed to create file watcher");

    // Watch the directory (parent of config file) to handle editors that use atomic saves (rename/move)
    let watch_target = path.parent().unwrap_or(&path);
    if let Err(e) = watcher.watch(watch_target, RecursiveMode::NonRecursive) {
        eprintln!("Failed to watch config directory: {}", e);
        return;
    }

    // Process events
    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => {
                // Check if the specific config file was modified/created
                let relevant = event.paths.iter().any(|p| p.ends_with("config.toml"));

                if relevant {
                    println!("Config file changed. Reloading...");
                    // Give fs a moment to settle (some editors write empty files first)
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

                    match fs::read_to_string(&path) {
                        Ok(content) => match toml::from_str::<Config>(&content) {
                            Ok(new_config) => {
                                if let Ok(mut w) = config_store.write() {
                                    *w = new_config;
                                }
                                let _ = sender.send(()).await;
                                println!("Config reloaded successfully.");
                            }
                            Err(e) => eprintln!("Config reload failed (Parse Error): {}", e),
                        },
                        Err(e) => eprintln!("Config reload failed (Read Error): {}", e),
                    }
                }
            }
            Err(e) => eprintln!("Watch error: {}", e),
        }
    }
}

