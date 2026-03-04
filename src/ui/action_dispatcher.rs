use crate::ui::menu_model::Action;

use super::radial::RadialMenu;

pub fn dispatch_action(menu: &RadialMenu, action: Action, x: f64, y: f64) {
    match action {
        Action::Activate {
            service,
            path,
            menu_path,
        } => dispatch_activate(service, path, menu_path, x, y),
        Action::Context {
            service, menu_path, ..
        } => dispatch_context(menu, service, menu_path),
        Action::DbusSignal { service, path, id } => dispatch_dbus_signal(service, path, id),
        Action::Command(cmd) => dispatch_command(cmd),
        Action::None => {}
    }
}

fn dispatch_activate(service: String, path: String, menu_path: String, x: f64, y: f64) {
    gtk4::glib::spawn_future_local(async move {
        let success = crate::tray::activate_or_popup(service, path, menu_path, x, y).await;

        if success {
            std::process::exit(0);
        }
    });
}

fn dispatch_context(menu: &RadialMenu, service: String, menu_path: String) {
    let menu_clone = menu.clone();
    gtk4::glib::spawn_future_local(async move {
        match crate::tray::fetch_dbus_menu_as_pie(service, menu_path).await {
            Ok(items) => {
                println!("Waypie: Context menu fetched with {} items", items.len());
                menu_clone.set_items(items);
            }
            Err(e) => eprintln!("Waypie: Failed to fetch context menu: {}", e),
        }
    });
}

fn dispatch_dbus_signal(service: String, path: String, id: i32) {
    crate::RUNTIME
        .get()
        .expect("Runtime not initialized")
        .spawn(async move {
            match zbus::Connection::session().await {
                Ok(conn) => {
                    let result = conn
                        .call_method(
                            Some(service.as_str()),
                            path.as_str(),
                            Some("com.canonical.dbusmenu"),
                            "Event",
                            &(id, "clicked", zbus::zvariant::Value::Str("".into()), 0u32),
                        )
                        .await;

                    match result {
                        Ok(_) => std::process::exit(0),
                        Err(e) => eprintln!("Waypie: DBus Event failed: {}", e),
                    }
                }
                Err(e) => {
                    eprintln!("Waypie: Failed to connect to session bus: {}", e)
                }
            }
        });
}

fn dispatch_command(cmd: String) {
    if !cmd.is_empty() {
        if let Err(e) = crate::utils::spawn_app(&cmd) {
            eprintln!("Waypie: Failed to execute command '{}': {}", cmd, e);
        } else {
            std::process::exit(0);
        }
    }
}
