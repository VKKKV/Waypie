pub mod watcher;
pub mod client;

// Re-exports
pub use watcher::{SNIWatcher, TrayItem, AppState};
pub use client::{activate_or_popup, fetch_dbus_menu_as_pie};
