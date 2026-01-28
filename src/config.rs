use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Debug, Clone)]
pub struct AppConfig {
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default)]
    pub items: Vec<MenuItemConfig>,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub actions: ActionConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MenuItemConfig {
    pub label: String,
    pub script: Option<String>,
    #[serde(default)]
    pub items: Vec<MenuItemConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct UiConfig {
    #[serde(default = "default_refresh_rate")]
    pub refresh_rate_ms: u64,
    #[serde(default = "default_size")]
    pub width: i32,
    #[serde(default = "default_size")]
    pub height: i32,
    #[serde(default = "default_outer_radius")]
    pub outer_radius: f64,
    #[serde(default = "default_tray_inner_radius")]
    pub tray_inner_radius: f64,
    #[serde(default = "default_vol_radius")]
    pub vol_radius: f64,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_hover_mode")]
    pub hover_mode: String,
    #[serde(default)]
    pub colors: ColorConfig,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            refresh_rate_ms: default_refresh_rate(),
            width: default_size(),
            height: default_size(),
            outer_radius: default_outer_radius(),
            tray_inner_radius: default_tray_inner_radius(),
            vol_radius: default_vol_radius(),
            font_family: default_font_family(),
            hover_mode: default_hover_mode(),
            colors: ColorConfig::default(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ColorConfig {
    #[serde(default = "default_bg_color")]
    pub background: (f64, f64, f64, f64), // r, g, b, a
    #[serde(default = "default_vol_track")]
    pub volume_track: (f64, f64, f64, f64),
    #[serde(default = "default_vol_color")]
    pub volume_arc: (f64, f64, f64),
    #[serde(default = "default_vol_warn")]
    pub volume_warning: (f64, f64, f64),
    #[serde(default = "default_text_color")]
    pub text: (f64, f64, f64),
    #[serde(default = "default_tray_even")]
    pub tray_even: (f64, f64, f64, f64),
    #[serde(default = "default_tray_odd")]
    pub tray_odd: (f64, f64, f64, f64),
    #[serde(default = "default_hover_overlay")]
    pub hover_overlay: (f64, f64, f64, f64),
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            background: default_bg_color(),
            volume_track: default_vol_track(),
            volume_arc: default_vol_color(),
            volume_warning: default_vol_warn(),
            text: default_text_color(),
            tray_even: default_tray_even(),
            tray_odd: default_tray_odd(),
            hover_overlay: default_hover_overlay(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ActionConfig {
    #[serde(default = "default_action_left_click")]
    pub left_click: Option<String>,
    #[serde(default = "default_action_right_click")]
    pub right_click: Option<String>,
    #[serde(default = "default_action_scroll_up")]
    pub scroll_up: Option<String>,
    #[serde(default = "default_action_scroll_down")]
    pub scroll_down: Option<String>,
}

impl Default for ActionConfig {
    fn default() -> Self {
        Self {
            left_click: default_action_left_click(),
            right_click: default_action_right_click(),
            scroll_up: default_action_scroll_up(),
            scroll_down: default_action_scroll_down(),
        }
    }
}

// Defaults
fn default_icon() -> String { "archlinux-logo".to_string() }
fn default_refresh_rate() -> u64 { 200 }
fn default_size() -> i32 { 600 }
fn default_outer_radius() -> f64 { 180.0 }
fn default_tray_inner_radius() -> f64 { 110.0 }
fn default_vol_radius() -> f64 { 95.0 }
fn default_font_family() -> String { "Sans".to_string() }
fn default_hover_mode() -> String { "highlight".to_string() }

fn default_bg_color() -> (f64, f64, f64, f64) { (0.1, 0.1, 0.1, 0.9) }
fn default_vol_track() -> (f64, f64, f64, f64) { (0.3, 0.3, 0.3, 0.5) }
fn default_vol_color() -> (f64, f64, f64) { (0.09, 0.57, 0.82) } // Arch Blue
fn default_vol_warn() -> (f64, f64, f64) { (0.8, 0.2, 0.2) } // Red
fn default_text_color() -> (f64, f64, f64) { (1.0, 1.0, 1.0) }
fn default_tray_even() -> (f64, f64, f64, f64) { (0.15, 0.15, 0.15, 0.9) }
fn default_tray_odd() -> (f64, f64, f64, f64) { (0.2, 0.2, 0.2, 0.9) }
fn default_hover_overlay() -> (f64, f64, f64, f64) { (1.0, 1.0, 1.0, 0.1) } // White 10% opacity

fn default_action_left_click() -> Option<String> { Some("pavucontrol".to_string()) } // Default: open volume control
fn default_action_right_click() -> Option<String> { None }
fn default_action_scroll_up() -> Option<String> { Some("pamixer -i 5".to_string()) }
fn default_action_scroll_down() -> Option<String> { Some("pamixer -d 5".to_string()) }

pub fn load() -> AppConfig {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("waypie");
    let config_path = xdg_dirs.find_config_file("config.toml");
    match config_path {
        Some(path) => {
            let content = fs::read_to_string(path).expect("Cannot read config");
            toml::from_str(&content).expect("Config format error")
        },
        None => {
            AppConfig {
                icon: default_icon(),
                items: vec![
                    MenuItemConfig { label: "Terminal".into(), script: Some("ghostty".into()), items: vec![] },
                    MenuItemConfig { label: "Browser".into(), script: Some("firefox".into()), items: vec![] }
                ],
                ui: UiConfig::default(),
                actions: ActionConfig::default(),
            }
        }
    }
}
