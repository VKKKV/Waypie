use crate::ui::menu_model::{Action, PieItem};

pub fn resolve_child_click_action(child: &PieItem, button: u32) -> Action {
    if should_open_context_for_child(child, button) {
        if let Action::Activate {
            service,
            path,
            menu_path,
        } = &child.action
        {
            return Action::Context {
                service: service.clone(),
                path: path.clone(),
                menu_path: menu_path.clone(),
            };
        }
    }

    child.action.clone()
}

pub fn should_open_context_for_child(child: &PieItem, button: u32) -> bool {
    child.item_type.as_deref() == Some("tray_app") && button == gtk4::gdk::BUTTON_SECONDARY
        || button == gtk4::gdk::BUTTON_PRIMARY
}

#[cfg(test)]
mod tests {
    use super::{resolve_child_click_action, should_open_context_for_child};
    use crate::ui::menu_model::{Action, PieItem};

    fn child_with_action(action: Action, item_type: Option<&str>) -> PieItem {
        PieItem {
            label: "item".to_string(),
            icon: "icon".to_string(),
            action,
            children: vec![],
            item_type: item_type.map(|s| s.to_string()),
            tray_id: None,
        }
    }

    #[test]
    fn primary_button_opens_context_for_activate_action() {
        let child = child_with_action(
            Action::Activate {
                service: "svc".to_string(),
                path: "/path".to_string(),
                menu_path: "/menu".to_string(),
            },
            Some("tray_app"),
        );

        let resolved = resolve_child_click_action(&child, gtk4::gdk::BUTTON_PRIMARY);
        assert!(matches!(resolved, Action::Context { .. }));
    }

    #[test]
    fn secondary_button_non_tray_keeps_original_action() {
        let child = child_with_action(Action::Command("alacritty".to_string()), None);

        let resolved = resolve_child_click_action(&child, gtk4::gdk::BUTTON_SECONDARY);
        assert_eq!(resolved, Action::Command("alacritty".to_string()));
    }

    #[test]
    fn context_opening_matches_existing_logic() {
        let tray_child = child_with_action(Action::None, Some("tray_app"));
        let non_tray_child = child_with_action(Action::None, None);

        assert!(should_open_context_for_child(
            &tray_child,
            gtk4::gdk::BUTTON_SECONDARY,
        ));
        assert!(should_open_context_for_child(
            &non_tray_child,
            gtk4::gdk::BUTTON_PRIMARY,
        ));
        assert!(!should_open_context_for_child(
            &non_tray_child,
            gtk4::gdk::BUTTON_SECONDARY,
        ));
    }

    #[test]
    fn non_primary_non_secondary_does_not_open_context() {
        let tray_child = child_with_action(Action::None, Some("tray_app"));
        assert!(!should_open_context_for_child(
            &tray_child,
            gtk4::gdk::BUTTON_MIDDLE,
        ));
    }
}
