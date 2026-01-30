use crate::color::{Color3, Color4, deserialize_color3, deserialize_color4};
use crate::utils::{hex_to_rgb, hex_to_rgba};
use serde::Deserialize;
use std::fs;

// Defaults - Constants for zero-copy and early evaluation
const DEFAULT_ICON: &str = "archlinux-logo";
const DEFAULT_REFRESH_RATE_MS: u64 = 200;
const DEFAULT_SIZE: i32 = 600;
const DEFAULT_OUTER_RADIUS: f64 = 180.0;
const DEFAULT_TRAY_INNER_RADIUS: f64 = 110.0;
const DEFAULT_VOL_RADIUS: f64 = 95.0;
const DEFAULT_FONT_FAMILY: &str = "Sans";
const DEFAULT_HOVER_MODE: &str = "highlight";

// Color defaults using hex notation (0xRRGGBBAA format)
const DEFAULT_BG_COLOR: Color4 = hex_to_rgba(0x1A1A1AE6);           // Dark with 90% alpha
const DEFAULT_VOL_TRACK: Color4 = hex_to_rgba(0x4D4D4D80);         // Grey with 50% alpha
const DEFAULT_VOL_COLOR: Color3 = hex_to_rgb(0x0E91D2);            // Arch Blue
const DEFAULT_VOL_WARN: Color3 = hex_to_rgb(0xCC3333);             // Red
const DEFAULT_TEXT_COLOR: Color3 = hex_to_rgb(0xFFFFFF);           // White
const DEFAULT_TRAY_EVEN: Color4 = hex_to_rgba(0x262626E6);         // Dark even rows
const DEFAULT_TRAY_ODD: Color4 = hex_to_rgba(0x333333E6);          // Dark odd rows
const DEFAULT_HOVER_OVERLAY: Color4 = hex_to_rgba(0xFFFFFF19);     // White 10% opacity

// Serde default function helpers - return constants without allocation
fn default_icon() -> String { DEFAULT_ICON.into() }
fn default_refresh_rate() -> u64 { DEFAULT_REFRESH_RATE_MS }
fn default_size() -> i32 { DEFAULT_SIZE }
fn default_outer_radius() -> f64 { DEFAULT_OUTER_RADIUS }
fn default_tray_inner_radius() -> f64 { DEFAULT_TRAY_INNER_RADIUS }
fn default_vol_radius() -> f64 { DEFAULT_VOL_RADIUS }
fn default_font_family() -> String { DEFAULT_FONT_FAMILY.into() }
fn default_hover_mode() -> String { DEFAULT_HOVER_MODE.into() }

fn default_bg_color() -> Color4 { DEFAULT_BG_COLOR }
fn default_vol_track() -> Color4 { DEFAULT_VOL_TRACK }
fn default_vol_color() -> Color3 { DEFAULT_VOL_COLOR }
fn default_vol_warn() -> Color3 { DEFAULT_VOL_WARN }
fn default_text_color() -> Color3 { DEFAULT_TEXT_COLOR }
fn default_tray_even() -> Color4 { DEFAULT_TRAY_EVEN }
fn default_tray_odd() -> Color4 { DEFAULT_TRAY_ODD }
fn default_hover_overlay() -> Color4 { DEFAULT_HOVER_OVERLAY }

fn default_action_left_click() -> Option<String> { Some("pwvucontrol".into()) }
fn default_action_right_click() -> Option<String> { Some("pwvucontrol".into()) }
fn default_action_scroll_up() -> Option<String> { Some("pamixer -i 5".into()) }
fn default_action_scroll_down() -> Option<String> { Some("pamixer -d 5".into()) }

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
    #[serde(default)]
    pub tray_apps: Vec<TrayAppConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MenuItemConfig {
    pub label: String,
    pub script: Option<String>,
    #[serde(default)]
    pub items: Vec<MenuItemConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TrayAppConfig {
    pub label: String,
    pub icon: String,
    #[serde(default)]
    pub actions: Vec<TrayActionConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TrayActionConfig {
    pub label: String,
    pub command: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
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

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ColorConfig {
    #[serde(default = "default_bg_color", deserialize_with = "deserialize_color4")]
    pub background: Color4,
    #[serde(default = "default_vol_track", deserialize_with = "deserialize_color4")]
    pub volume_track: Color4,
    #[serde(default = "default_vol_color", deserialize_with = "deserialize_color3")]
    pub volume_arc: Color3,
    #[serde(default = "default_vol_warn", deserialize_with = "deserialize_color3")]
    pub volume_warning: Color3,
    #[serde(default = "default_text_color", deserialize_with = "deserialize_color3")]
    pub text: Color3,
    #[serde(default = "default_tray_even", deserialize_with = "deserialize_color4")]
    pub tray_even: Color4,
    #[serde(default = "default_tray_odd", deserialize_with = "deserialize_color4")]
    pub tray_odd: Color4,
    #[serde(default = "default_hover_overlay", deserialize_with = "deserialize_color4")]
    pub hover_overlay: Color4,
}

#[derive(Deserialize, Debug, Clone, Default)]
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


pub fn load() -> AppConfig {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("waypie");
    match xdg_dirs.find_config_file("config.toml") {
        Some(path) => toml::from_str(&fs::read_to_string(path).expect("Cannot read config"))
            .expect("Config format error"),
        None => AppConfig {
            icon: default_icon(),
            items: vec![
                MenuItemConfig {
                    label: "Terminal".into(),
                    script: Some("ghostty".into()),
                    items: vec![],
                },
                MenuItemConfig {
                    label: "Browser".into(),
                    script: Some("firefox".into()),
                    items: vec![],
                },
            ],
            ui: UiConfig::default(),
            actions: ActionConfig::default(),
            tray_apps: vec![],
        },
    }
}
