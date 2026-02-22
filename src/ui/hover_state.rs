/// Hover state and detection logic
/// Responsible for determining which menu item is under the cursor

use crate::ui::menu_model::PieItem;

pub struct HoverState {
    pub parent_idx: Option<usize>,
    pub child_idx: Option<usize>,
    pub active_parent_idx: Option<usize>,
}

impl HoverState {
    pub fn new() -> Self {
        HoverState {
            parent_idx: None,
            child_idx: None,
            active_parent_idx: None,
        }
    }

    pub fn reset_hover(&mut self) {
        self.parent_idx = None;
        self.child_idx = None;
    }

    pub fn reset_all(&mut self) {
        self.parent_idx = None;
        self.child_idx = None;
        self.active_parent_idx = None;
    }
}

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
    items
        .get(parent_idx)
        .map(|p| p.children.len())
        .unwrap_or(0)
}
