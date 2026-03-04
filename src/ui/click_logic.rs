use crate::ui::menu_model::{Action, PieItem};

pub fn resolve_clicked_action(
    items: &[PieItem],
    hover_child_idx: Option<usize>,
    active_parent_idx: Option<usize>,
    hover_parent_idx: Option<usize>,
    button: u32,
) -> Option<Action> {
    if let Some(child_idx) = hover_child_idx {
        if let Some(active_idx) = active_parent_idx {
            if let Some(parent) = items.get(active_idx) {
                if let Some(child) = parent.children.get(child_idx) {
                    return Some(resolve_child_click_action(child, button));
                }
            }
        }
    }

    if let Some(parent_idx) = hover_parent_idx {
        if let Some(parent) = items.get(parent_idx) {
            if parent.children.is_empty() {
                return Some(parent.action.clone());
            }
        }
    }

    None
}

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
    use super::{
        resolve_child_click_action, resolve_clicked_action, should_open_context_for_child,
    };
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

    #[test]
    fn resolve_clicked_action_prefers_hovered_child() {
        let child = child_with_action(Action::Command("cmd".to_string()), None);
        let parent = PieItem {
            label: "parent".to_string(),
            icon: "icon".to_string(),
            action: Action::Command("parent-cmd".to_string()),
            children: vec![child],
            item_type: None,
            tray_id: None,
        };

        let action = resolve_clicked_action(
            &[parent],
            Some(0),
            Some(0),
            Some(0),
            gtk4::gdk::BUTTON_PRIMARY,
        );

        assert_eq!(action, Some(Action::Command("cmd".to_string())));
    }

    #[test]
    fn resolve_clicked_action_falls_back_to_leaf_parent() {
        let parent = child_with_action(Action::Command("parent-cmd".to_string()), None);

        let action =
            resolve_clicked_action(&[parent], None, None, Some(0), gtk4::gdk::BUTTON_PRIMARY);

        assert_eq!(action, Some(Action::Command("parent-cmd".to_string())));
    }
}
