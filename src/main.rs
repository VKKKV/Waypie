use gtk4::prelude::*;
use gtk4::Application;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

mod config;
mod color;
mod hud;
mod sni_watcher;
mod utils;
mod ui;
mod cursor;
mod dbus_menu;

const APP_ID: &str = "com.arch.waypie";

pub static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn main() {
    RUNTIME.set(Runtime::new().expect("Failed to create Tokio runtime")).expect("Failed to set global runtime");

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(ui::build_ui);
    app.run();
}