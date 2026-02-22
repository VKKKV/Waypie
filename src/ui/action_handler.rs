/// Action handler - executes menu item actions
/// Decoupled from UI to allow testing and reuse

use crate::ui::menu_model::Action;

/// Execute a shell command action
pub fn handle_command_action(cmd: &str) -> Result<(), String> {
    if cmd.is_empty() {
        return Ok(());
    }
    crate::utils::spawn_app(cmd).map_err(|e| e.to_string())
}

/// Handle activate action for tray items
/// Returns true if activation was successful
pub async fn handle_activate_action(
    service: String,
    path: String,
    menu_path: String,
    x: f64,
    y: f64,
) -> bool {
    let service_clone = service.clone();
    let x_int = x as i32;
    let y_int = y as i32;

    println!(
        "Waypie: Attempting Activate for {} at {}...",
        service, path
    );

    let client = if let Some(state) = crate::APP_STATE.get() {
        state.client.lock().unwrap().clone()
    } else {
        println!("Waypie: Client not initialized");
        return false;
    };

    let (tx, rx) = tokio::sync::oneshot::channel();

    crate::RUNTIME
        .get()
        .expect("Runtime not initialized")
        .spawn(async move {
            let result = async {
                if let Some(client) = client {
                    let req = system_tray::client::ActivateRequest::Default {
                        address: service,
                        x: x_int,
                        y: y_int,
                    };
                    return client.activate(req).await.map_err(|e| e.to_string());
                }
                Err("Waypie: Client not initialized".to_string())
            }
            .await;

            let _ = tx.send(result);
        });

    let activate_result = match rx.await {
        Ok(res) => res,
        Err(_) => Err("Tokio task cancelled".to_string()),
    };

    println!(
        "Waypie: Activate Result for {}: {:?}",
        service_clone, activate_result
    );

    activate_result.is_ok()
}

/// Handle context menu action for tray items
pub async fn handle_context_action(
    service: String,
    menu_path: String,
) -> Result<Vec<crate::ui::menu_model::PieItem>, String> {
    crate::tray::fetch_dbus_menu_as_pie(service, menu_path).await
}

/// Handle DBus menu signal action
pub async fn handle_dbus_signal_action(
    service: String,
    path: String,
    id: i32,
) -> Result<(), String> {
    let conn = zbus::Connection::session()
        .await
        .map_err(|e| format!("Failed to connect to session bus: {}", e))?;

    conn.call_method(
        Some(service.as_str()),
        path.as_str(),
        Some("com.canonical.dbusmenu"),
        "Event",
        &(
            id,
            "clicked",
            zbus::zvariant::Value::Str("".into()),
            0u32,
        ),
    )
    .await
    .map_err(|e| format!("DBus Event failed: {}", e))
    .map(|_| ())
}

/// Execute an action and handle the result
/// This is the main entry point for action execution
pub async fn execute_action(action: &Action, x: f64, y: f64) -> bool {
    match action {
        Action::Command(cmd) => {
            match handle_command_action(cmd) {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("Waypie: Failed to execute command '{}': {}", cmd, e);
                    false
                }
            }
        }
        Action::Activate {
            service,
            path,
            menu_path,
        } => {
            handle_activate_action(service.clone(), path.clone(), menu_path.clone(), x, y).await
        }
        Action::Context {
            service,
            menu_path,
            ..
        } => {
            match handle_context_action(service.clone(), menu_path.clone()).await {
                Ok(items) => {
                    println!("Waypie: Context menu fetched with {} items", items.len());
                    true
                }
                Err(e) => {
                    eprintln!("Waypie: Failed to fetch context menu: {}", e);
                    false
                }
            }
        }
        Action::DbusSignal { service, path, id } => {
            match handle_dbus_signal_action(service.clone(), path.clone(), *id).await {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("Waypie: DBus Event failed: {}", e);
                    false
                }
            }
        }
        Action::None => true,
    }
}
