use crate::config::MenuItemConfig;
use crate::tray::TrayItem;
use crate::ui::menu_model::{Action, PieItem};

/// Parse a string action from config into an Action enum
fn parse_action_string(s: String) -> Action {
    if s.is_empty() {
        return Action::None;
    }

    if let Some(rest) = s.strip_prefix("activate|") {
        let mut parts = rest.splitn(3, '|');
        if let (Some(service), Some(path), Some(menu_path)) =
            (parts.next(), parts.next(), parts.next())
        {
            return Action::Activate {
                service: service.to_string(),
                path: path.to_string(),
                menu_path: menu_path.to_string(),
            };
        }
    } else if let Some(rest) = s.strip_prefix("context|") {
        let mut parts = rest.splitn(3, '|');
        if let (Some(service), Some(path), Some(menu_path)) =
            (parts.next(), parts.next(), parts.next())
        {
            return Action::Context {
                service: service.to_string(),
                path: path.to_string(),
                menu_path: menu_path.to_string(),
            };
        }
    } else if let Some(rest) = s.strip_prefix("dbus_signal|") {
        let mut parts = rest.splitn(3, '|');
        if let (Some(service), Some(path), Some(id_part)) =
            (parts.next(), parts.next(), parts.next())
        {
            if let Ok(id) = id_part.parse::<i32>() {
                return Action::DbusSignal {
                    service: service.to_string(),
                    path: path.to_string(),
                    id,
                };
            }
        }
    }

    Action::Command(s)
}

pub fn convert_menu_items(items: &[MenuItemConfig], tray_items: &[TrayItem]) -> Vec<PieItem> {
    items
        .iter()
        .map(|item| {
            if item.item_type.as_deref() == Some("tray") {
                let mut tray_children = Vec::with_capacity(tray_items.len().max(1));

                if tray_items.is_empty() {
                    tray_children.push(PieItem {
                        label: "Empty".to_string(),
                        icon: "emblem-important".to_string(),
                        action: Action::None,
                        children: vec![],
                        item_type: None,
                        tray_id: None,
                    });
                } else {
                    for tray in tray_items {
                        let activate_action = Action::Activate {
                            service: tray.service.clone(),
                            path: tray.path.clone(),
                            menu_path: tray.menu_path.clone(),
                        };
                        let context_action = Action::Context {
                            service: tray.name.clone(),
                            path: tray.path.clone(),
                            menu_path: tray.menu_path.clone(),
                        };

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
                    action: parse_action_string(item.action.clone()),
                    children: tray_children,
                    item_type: item.item_type.clone(),
                    tray_id: None,
                }
            } else {
                PieItem {
                    label: item.label.clone(),
                    icon: item.icon.clone(),
                    action: parse_action_string(item.action.clone()),
                    children: convert_menu_items(&item.children, tray_items),
                    item_type: item.item_type.clone(),
                    tray_id: None,
                }
            }
        })
        .collect()
}
