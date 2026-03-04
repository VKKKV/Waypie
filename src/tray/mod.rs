pub mod client;
pub mod watcher;

// Re-exports
pub use client::{activate_or_popup, fetch_dbus_menu_as_pie};
pub use watcher::{AppState, SNIWatcher, TrayItem};
