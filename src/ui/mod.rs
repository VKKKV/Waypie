pub mod window;
pub mod radial;
pub mod radial_imp;
pub mod adapter;
pub mod menu_model;
pub mod hover_state;

use gtk4::prelude::*;
use gtk4::Application;
use std::sync::{Arc, RwLock};

use crate::config;
use crate::tray::SNIWatcher;

pub use radial::RadialMenu;
pub use adapter::convert_menu_items;

pub fn build_ui(app: &Application) {
    // 1. Load Initial Config
    let initial_config = config::load_config();
    let config_store = Arc::new(RwLock::new(initial_config.clone()));

    // 2. Setup Tokio Runtime & Config Watcher
    let (sender, receiver) = async_channel::unbounded::<()>();
    let config_store_clone = config_store.clone();

    let sni = Arc::new(SNIWatcher::new(Some(sender.clone())));
    let _ = crate::APP_STATE.set(sni.state.clone());
    let sni_clone = sni.clone();

    // Spawn on global runtime
    crate::RUNTIME
        .get()
        .expect("Runtime not initialized")
        .spawn(async move {
            tokio::join!(config::watch_config(config_store_clone, sender), async {
                let _ = sni_clone.start().await;
            });
        });

    // 3. Setup Main Window
    let window = window::build_window(app, &initial_config.ui);

    // 4. Create Radial Menu
    let radial_menu = RadialMenu::new();
    radial_menu.set_ui_config(initial_config.ui.clone());

    // Convert and set initial items
    let initial_tray_list = sni.get_legacy_items();
    let pie_items = convert_menu_items(&initial_config.menu, &initial_tray_list);
    radial_menu.set_items(pie_items);

    // Exit on Escape
    let controller = gtk4::EventControllerKey::new();
    controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Escape {
            std::process::exit(0);
        }
        gtk4::glib::Propagation::Stop
    });
    window.add_controller(controller);

    window.set_child(Some(&radial_menu));
    window.present();

    // Focus cursor to center
    let window_weak = window.downgrade();
    gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        if let Some(win) = window_weak.upgrade() {
            let width = win.width();
            let height = win.height();
            if width > 0 && height > 0 {
                crate::cursor::move_cursor_to_center(width as u32, height as u32);
            }
        }
        gtk4::glib::ControlFlow::Break
    });

    // 5. Handle Config Updates
    let menu_weak = radial_menu.downgrade();
    let store_weak = Arc::downgrade(&config_store);
    let sni_weak = Arc::downgrade(&sni);

    gtk4::glib::spawn_future_local(async move {
        while receiver.recv().await.is_ok() {
            if let Some(menu) = menu_weak.upgrade() {
                if let Some(store) = store_weak.upgrade() {
                    if let Ok(cfg) = store.read() {
                        let current_tray_items = if let Some(sni_up) = sni_weak.upgrade() {
                            sni_up.get_legacy_items()
                        } else {
                            Vec::new()
                        };
                        let new_items = convert_menu_items(&cfg.menu, &current_tray_items);
                        menu.set_items(new_items);
                        menu.set_ui_config(cfg.ui.clone());
                    }
                }
            }
        }
    });
}
