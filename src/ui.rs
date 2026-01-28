use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::sync::{Arc, RwLock};

use crate::config::{self, MenuItemConfig};
use crate::hud::radial_menu::{PieItem, RadialMenu};
use crate::sni_watcher::{SNIWatcher, TrayItem};

pub fn build_ui(app: &Application) {
    // 1. Load Initial Config
    let initial_config = config::load_config();
    let config_store = Arc::new(RwLock::new(initial_config.clone()));

    // 2. Setup Tokio Runtime & Config Watcher
    let (sender, receiver) = async_channel::unbounded::<()>();
    let config_store_clone = config_store.clone();

    let sni = SNIWatcher::new(Some(sender.clone()));
    let tray_items = sni.items();

    // Spawn on global runtime
    crate::RUNTIME
        .get()
        .expect("Runtime not initialized")
        .spawn(async move {
            tokio::join!(config::watch_config(config_store_clone, sender), async {
                let _ = sni.start().await;
            });
        });

    // 3. Setup Main Window
    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(initial_config.ui.width)
        .default_height(initial_config.ui.height)
        .build();

    // Layer Shell
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Bottom, true);

    // Transparency
    let provider = CssProvider::new();
    provider.load_from_data("window { background-color: transparent; }");
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("No display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // 4. Create Radial Menu
    let radial_menu = RadialMenu::new();
    radial_menu.set_tray_items(tray_items.clone());
    radial_menu.set_ui_config(initial_config.ui.clone());

    // Convert and set initial items
    let initial_tray_list = if let Ok(items) = tray_items.lock() {
        items.clone()
    } else {
        Vec::new()
    };
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
                // println!("Moving cursor to center: {}x{}", width, height);
                crate::cursor::move_cursor_to_center(width as u32, height as u32);
            }
        }
        gtk4::glib::ControlFlow::Break
    });

    // 5. Handle Config Updates
    let menu_weak = radial_menu.downgrade();
    let store_weak = Arc::downgrade(&config_store);
    let tray_items_clone = tray_items.clone();

    gtk4::glib::spawn_future_local(async move {
        while let Ok(_) = receiver.recv().await {
            if let Some(menu) = menu_weak.upgrade() {
                if let Some(store) = store_weak.upgrade() {
                    if let Ok(cfg) = store.read() {
                        let current_tray_items = if let Ok(items) = tray_items_clone.lock() {
                            items.clone()
                        } else {
                            Vec::new()
                        };
                        let new_items = convert_menu_items(&cfg.menu, &current_tray_items);
                        menu.set_items(new_items);
                        menu.set_ui_config(cfg.ui.clone());
                        println!("UI updated with new config.");
                    }
                }
            }
        }
    });
}

fn convert_menu_items(items: &[MenuItemConfig], tray_items: &[TrayItem]) -> Vec<PieItem> {
    items
        .iter()
        .map(|item| {
            if item.item_type.as_deref() == Some("tray") {
                let mut tray_children = Vec::new();

                if tray_items.is_empty() {
                    tray_children.push(PieItem {
                        label: "Empty".to_string(),
                        icon: "emblem-important".to_string(),
                        action: "".to_string(),
                        children: vec![],
                        item_type: None,
                        tray_id: None,
                    });
                } else {
                    for tray in tray_items {
                        let activate_action =
                            format!("activate|{}|{}|{}", tray.service, tray.path, tray.menu_path);
                        let context_action =
                            format!("context|{}|{}|{}", tray.service, tray.path, tray.menu_path);

                        tray_children.push(PieItem {
                            label: tray.title.clone(),
                            icon: tray.icon_name.clone(),
                            action: activate_action,
                            children: vec![PieItem {
                                label: "Context Menu".to_string(),
                                icon: "view-more-symbolic".to_string(),
                                action: context_action,
                                children: vec![],
                                item_type: Some("tray_context".to_string()),
                                tray_id: None,
                            }],
                            item_type: Some("tray_app".to_string()),
                            tray_id: Some(format!("{}|{}", tray.service, tray.path)),
                        });
                    }
                }

                PieItem {
                    label: item.label.clone(),
                    icon: item.icon.clone(),
                    action: item.action.clone(),
                    children: tray_children,
                    item_type: item.item_type.clone(),
                    tray_id: None,
                }
            } else {
                PieItem {
                    label: item.label.clone(),
                    icon: item.icon.clone(),
                    action: item.action.clone(),
                    children: convert_menu_items(&item.children, tray_items),
                    item_type: item.item_type.clone(),
                    tray_id: None,
                }
            }
        })
        .collect()
}
