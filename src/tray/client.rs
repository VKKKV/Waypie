use system_tray::client::ActivateRequest;
use system_tray::menu::{MenuItem, MenuType, TrayMenu};

use crate::ui::menu_model::{Action, PieItem};

/// Tries to activate the item via SNI `Activate` method using system-tray client.
/// Returns `true` if activation succeeded.
pub async fn activate_or_popup(
    service: String,
    item_path: String,
    _menu_path: String,
    _parent_widget: gtk4::Widget,
    x: f64,
    y: f64,
) -> bool {
    let service_clone = service.clone();
    let x_int = x as i32;
    let y_int = y as i32;

    println!(
        "Waypie: Attempting Activate for {} at {}...",
        service, item_path
    );

    // Clone the client before spawning
    let client = if let Some(state) = crate::APP_STATE.get() {
        state.client.lock().unwrap().clone()
    } else {
        println!("Waypie: Client not initialized");
        return false;
    };

    let activate_result = if let Some(client) = client {
        let req = ActivateRequest::Default {
            address: service,
            x: x_int,
            y: y_int,
        };
        client.activate(req).await.map_err(|e| e.to_string())
    } else {
        Err("Waypie: Client not initialized".to_string())
    };

    println!(
        "Waypie: Activate Result for {}: {:?}",
        service_clone, activate_result
    );

    activate_result.is_ok()
}

/// Converts system-tray MenuItem to PieItem recursively
pub fn convert_menu_item_to_pie(item: &MenuItem, service: &str, path: &str) -> Option<PieItem> {
    if !item.visible {
        return None;
    }

    if matches!(item.menu_type, MenuType::Separator) {
        return None;
    }

    let label = item
        .label
        .as_ref()
        .cloned()
        .unwrap_or_default()
        .replace('_', "");
    if label.is_empty() && item.submenu.is_empty() {
        return None;
    }

    let icon = item
        .icon_name
        .clone()
        .unwrap_or_else(|| "view-more-symbolic".to_string());

    let action = Action::DbusSignal {
        service: service.to_string(),
        path: path.to_string(),
        id: item.id,
    };

    let mut children = Vec::with_capacity(item.submenu.len());
    for child in &item.submenu {
        if let Some(pie_child) = convert_menu_item_to_pie(child, service, path) {
            children.push(pie_child);
        }
    }

    Some(PieItem {
        label,
        icon,
        action,
        children,
        item_type: Some("dbus_item".to_string()),
        tray_id: None,
    })
}

pub fn convert_tray_menu_to_pie(menu: &TrayMenu, service: &str, path: &str) -> Vec<PieItem> {
    let mut items = Vec::with_capacity(menu.submenus.len());
    for item in &menu.submenus {
        if let Some(pie_item) = convert_menu_item_to_pie(item, service, path) {
            items.push(pie_item);
        }
    }
    items
}

pub async fn fetch_dbus_menu_as_pie(name: String, path: String) -> Result<Vec<PieItem>, String> {
    let (service, _item_path) = name.split_once('/').unwrap_or((&name, ""));
    let service = service.to_string();

    let client = if let Some(state) = crate::APP_STATE.get() {
        let stored = state.client.lock().unwrap();
        stored
            .clone()
            .ok_or_else(|| "Waypie: [Error] Client not initialized".to_string())?
    } else {
        return Err("Waypie: [Error] AppState not initialized".to_string());
    };

    let _ = client
        .about_to_show_menuitem(service.clone(), path.clone(), 0)
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let menu = if let Some(state) = crate::APP_STATE.get() {
        let store = state.items.lock().unwrap();
        store.get(&name).and_then(|(_, menu)| menu.clone())
    } else {
        None
    };

    if let Some(menu) = menu {
        return Ok(convert_tray_menu_to_pie(&menu, &service, &path));
    }

    Err(format!(
        "Waypie: [Error] Menu not found in cache for {}",
        name
    ))
}
