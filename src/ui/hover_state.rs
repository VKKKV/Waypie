/// Hover state and detection logic
/// Responsible for determining which menu item is under the cursor
use crate::ui::menu_model::PieItem;

/// Normalize angle to 0-360 range with 90° offset
pub fn normalize_angle(angle: f64) -> f64 {
    let mut normalized = angle + 90.0;
    if normalized < 0.0 {
        normalized += 360.0;
    }
    normalized
}

/// Calculate which item is hovered based on angle and item count
pub fn calculate_hovered_item(angle: f64, item_count: usize) -> Option<usize> {
    if item_count == 0 {
        return None;
    }
    let angle_per_item = 360.0 / item_count as f64;
    let idx = (angle / angle_per_item).floor() as usize;
    Some(idx.min(item_count - 1))
}

/// Determine hover state based on distance and angle
pub enum HoverZone {
    Center,
    InnerRing,
    OuterRing,
    Outside,
}

pub fn get_hover_zone(
    dist: f64,
    center_radius: f64,
    inner_radius: f64,
    outer_radius: f64,
) -> HoverZone {
    if dist <= center_radius {
        HoverZone::Center
    } else if dist <= inner_radius {
        HoverZone::InnerRing
    } else if dist >= inner_radius + 10.0 && dist <= outer_radius {
        HoverZone::OuterRing
    } else {
        HoverZone::Outside
    }
}

/// Get child count from items list for a given parent index
pub fn get_child_count(items: &[PieItem], parent_idx: usize) -> usize {
    items.get(parent_idx).map(|p| p.children.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        calculate_hovered_item, get_child_count, get_hover_zone, normalize_angle, HoverZone,
    };
    use crate::ui::menu_model::{Action, PieItem};

    #[test]
    fn normalize_angle_offsets_by_ninety() {
        assert_eq!(normalize_angle(0.0), 90.0);
        assert_eq!(normalize_angle(-90.0), 0.0);
    }

    #[test]
    fn normalize_angle_wraps_negative_values() {
        assert_eq!(normalize_angle(-180.0), 270.0);
        assert_eq!(normalize_angle(-360.0), 90.0);
    }

    #[test]
    fn hovered_item_handles_empty_and_basic_partitioning() {
        assert_eq!(calculate_hovered_item(10.0, 0), None);
        assert_eq!(calculate_hovered_item(0.0, 4), Some(0));
        assert_eq!(calculate_hovered_item(89.0, 4), Some(0));
        assert_eq!(calculate_hovered_item(90.0, 4), Some(1));
        assert_eq!(calculate_hovered_item(359.0, 4), Some(3));
    }

    #[test]
    fn hover_zone_respects_boundaries() {
        assert!(matches!(
            get_hover_zone(100.0, 100.0, 250.0, 400.0),
            HoverZone::Center
        ));
        assert!(matches!(
            get_hover_zone(200.0, 100.0, 250.0, 400.0),
            HoverZone::InnerRing
        ));
        assert!(matches!(
            get_hover_zone(260.0, 100.0, 250.0, 400.0),
            HoverZone::OuterRing
        ));
        assert!(matches!(
            get_hover_zone(255.0, 100.0, 250.0, 400.0),
            HoverZone::Outside
        ));
        assert!(matches!(
            get_hover_zone(410.0, 100.0, 250.0, 400.0),
            HoverZone::Outside
        ));
    }

    #[test]
    fn child_count_handles_missing_parent() {
        let items = vec![PieItem {
            label: "Parent".to_string(),
            icon: "icon".to_string(),
            action: Action::None,
            children: vec![PieItem {
                label: "Child".to_string(),
                icon: "icon".to_string(),
                action: Action::None,
                children: vec![],
                item_type: None,
                tray_id: None,
            }],
            item_type: None,
            tray_id: None,
        }];

        assert_eq!(get_child_count(&items, 0), 1);
        assert_eq!(get_child_count(&items, 1), 0);
    }
}
