use crate::config::MenuItemConfig;
use crate::ui::radial::PieItem;
use crate::tray::TrayItem;

pub fn convert_menu_items(items: &[MenuItemConfig], tray_items: &[TrayItem]) -> Vec<PieItem> {
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
                            format!("context|{}|{}|{}", tray.name, tray.path, tray.menu_path);

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
