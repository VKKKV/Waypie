use gtk4::prelude::*;
use gtk4::Application;
use std::sync::{Arc, OnceLock};
use tokio::runtime::Runtime;

mod color;
mod config;
mod cursor;
mod tray;
mod ui;
mod utils;

const APP_ID: &str = "com.arch.waypie";

pub static RUNTIME: OnceLock<Runtime> = OnceLock::new();
pub static APP_STATE: OnceLock<Arc<tray::AppState>> = OnceLock::new();

fn main() {
    RUNTIME
        .set(Runtime::new().expect("Failed to create Tokio runtime"))
        .expect("Failed to set global runtime");

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(ui::build_ui);
    app.run();
}
