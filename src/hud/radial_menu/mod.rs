use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::GestureClick;
use std::time::{Duration, Instant};

// 1. Data Structure
#[derive(Clone, Debug)]
pub struct PieItem {
    pub label: String,
    pub icon: String,
    pub action: String,
    pub children: Vec<PieItem>,
    pub item_type: Option<String>,
    pub tray_id: Option<String>,
}

mod imp;

use crate::sni_watcher::TrayItems;

glib::wrapper! {
    pub struct RadialMenu(ObjectSubclass<imp::RadialMenu>)
        @extends gtk4::Widget;
}

impl RadialMenu {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_items(&self, items: Vec<PieItem>) {
        self.imp().items.replace(items);
        self.queue_draw();
    }

    pub fn set_tray_items(&self, items: TrayItems) {
        self.imp().tray_items.replace(Some(items));
    }

    pub fn set_ui_config(&self, config: crate::config::UiConfig) {
        self.imp().ui_config.replace(config);
        self.queue_draw();
    }

    fn get_child_count(&self, parent_idx: usize) -> usize {
        let imp = self.imp();
        let items = imp.items.borrow();
        if let Some(parent) = items.get(parent_idx) {
            return parent.children.len();
        }
        0
    }

    fn handle_motion(&self, x: f64, y: f64) {
        let imp = self.imp();
        let w = self.width() as f64;
        let h = self.height() as f64;
        let cx = w / 2.0;
        let cy = h / 2.0;

        let (dist, angle_deg) = self.cartesian_to_polar(x, y, cx, cy);
        let items = imp.items.borrow();
        let parent_count = items.len();

        if parent_count == 0 {
            return;
        }

        let mut should_redraw = false;
        let mut reset_hover_child = true;
        let ui = imp.ui_config.borrow();
        let center_radius = ui.center_radius;
        let inner_radius_end = ui.inner_radius;
        let outer_radius_start = ui.inner_radius + 10.0;
        let outer_radius_end = ui.outer_radius;

        // --- Logic: Center ---
        if dist <= center_radius {
            if imp.hover_parent_idx.get().is_some() {
                imp.hover_parent_idx.set(None);
                imp.hover_start_time.set(None);
                should_redraw = true;
            }
            // Ensure child hover is also reset if we move to center
            reset_hover_child = true;
        }
        // --- Logic: Inner Ring (Parents) ---
        else if dist <= inner_radius_end {
            let angle_per_item = 360.0 / parent_count as f64;

            let mut norm_angle = angle_deg + 90.0;
            if norm_angle < 0.0 {
                norm_angle += 360.0;
            }

            let idx = (norm_angle / angle_per_item).floor() as usize;
            let idx = idx.min(parent_count - 1);

            let current_hover = imp.hover_parent_idx.get();
            if current_hover != Some(idx) {
                imp.hover_parent_idx.set(Some(idx));
                imp.hover_start_time.set(Some(Instant::now()));
                should_redraw = true;
            }

            reset_hover_child = true;
        }
        // --- Logic: Outer Ring (Children) ---
        else if dist >= outer_radius_start && dist <= outer_radius_end {
            // Only if active parent exists
            if let Some(active_idx) = imp.active_parent_idx.get() {
                let child_count = self.get_child_count(active_idx);

                if child_count > 0 {
                    let angle_per_child = 360.0 / child_count as f64;
                    let mut norm_angle = angle_deg + 90.0;
                    if norm_angle < 0.0 {
                        norm_angle += 360.0;
                    }

                    let idx = (norm_angle / angle_per_child).floor() as usize;
                    let idx = idx.min(child_count - 1);

                    if imp.hover_child_idx.get() != Some(idx) {
                        imp.hover_child_idx.set(Some(idx));
                        should_redraw = true;
                    }
                    reset_hover_child = false;
                }
            } else {
                if imp.hover_parent_idx.get().is_some() {
                    imp.hover_parent_idx.set(None);
                    imp.hover_start_time.set(None);
                    should_redraw = true;
                }
            }
        }
        // --- Logic: Outside or Dead Zone ---
        else {
            if imp.hover_parent_idx.get().is_some() {
                imp.hover_parent_idx.set(None);
                imp.hover_start_time.set(None);
                should_redraw = true;
            }
        }

        if reset_hover_child && imp.hover_child_idx.get().is_some() {
            imp.hover_child_idx.set(None);
            should_redraw = true;
        }

        if should_redraw {
            self.queue_draw();
        }
    }

    fn handle_leave(&self) {
        let imp = self.imp();
        imp.hover_parent_idx.set(None);
        imp.hover_child_idx.set(None);

        // Don't reset active parent immediately on leave if you want it to stay open?
        // Prompt says "If mouse leaves logic area: Reset state."
        imp.active_parent_idx.set(None);
        imp.target_progress.set(0.0); // Animate out

        self.queue_draw();
    }

    fn check_hover_timer(&self) {
        let imp = self.imp();

        if let Some(hover_idx) = imp.hover_parent_idx.get() {
            if Some(hover_idx) != imp.active_parent_idx.get() {
                if let Some(start_time) = imp.hover_start_time.get() {
                    if start_time.elapsed() >= Duration::from_millis(100) {
                        imp.active_parent_idx.set(Some(hover_idx));

                        // Fix Ghost Ring: Check if children exist
                        let child_count = self.get_child_count(hover_idx);
                        let target = if child_count > 0 { 1.0 } else { 0.0 };

                        if (imp.target_progress.get() - target).abs() > 0.001 {
                            imp.target_progress.set(target);
                            self.queue_draw();
                        } else {
                            // If we switched active parent but animation target is same (e.g. 1.0 -> 1.0 or 0.0 -> 0.0),
                            // we still need to redraw to show the new active parent highlight
                            self.queue_draw();
                        }
                    }
                }
            }
        }
    }

    fn handle_click(&self, gesture: &GestureClick, _n_press: i32, x: f64, y: f64) {
        let imp = self.imp();
        let w = self.width() as f64;
        let h = self.height() as f64;
        let cx = w / 2.0;
        let cy = h / 2.0;

        let (dist, _angle_deg) = self.cartesian_to_polar(x, y, cx, cy);
        let items = imp.items.borrow();
        let button = gesture.current_button();

        // 1. Check Center Click
        let ui = imp.ui_config.borrow();
        if dist < ui.center_radius {
            std::process::exit(0);
        }

        // 2. Determine Action
        let mut clicked_action = None;

        if let Some(child_idx) = imp.hover_child_idx.get() {
            if let Some(active_idx) = imp.active_parent_idx.get() {
                if let Some(parent) = items.get(active_idx) {
                    if let Some(child) = parent.children.get(child_idx) {
                        if child.item_type.as_deref() == Some("tray_app")
                            && button == gtk4::gdk::BUTTON_SECONDARY
                        {
                            if child.action.starts_with("activate|") {
                                clicked_action =
                                    Some(child.action.replace("activate|", "context|"));
                            } else {
                                clicked_action = Some(child.action.clone());
                            }
                        } else {
                            clicked_action = Some(child.action.clone());
                        }
                    }
                }
            }
        } else if let Some(parent_idx) = imp.hover_parent_idx.get() {
            if let Some(parent) = items.get(parent_idx) {
                if parent.children.is_empty() {
                    clicked_action = Some(parent.action.clone());
                }
            }
        }

        // 3. Execute Action
        if let Some(action) = clicked_action {
            if !action.is_empty() {
                let action_clone = action.clone();

                if action_clone.starts_with("activate|") {
                    let self_clone = self.clone();
                    gtk4::glib::spawn_future_local(async move {
                        let parts: Vec<&str> = action_clone.splitn(4, '|').collect();
                        if parts.len() == 4 {
                            let service = parts[1].to_string();
                            let path = parts[2].to_string();
                            let menu_path = parts[3].to_string();
                            
                            // Try Activate, Fallback to Popup
                            let success = crate::dbus_menu::activate_or_popup(
                                service, 
                                path, 
                                menu_path, 
                                self_clone.upcast_ref::<gtk4::Widget>().clone(), 
                                x, 
                                y
                            ).await;

                            if success {
                                std::process::exit(0);
                            }
                        } else {
                            // Fallback for old/broken config or tray items?
                            // Or just log error.
                            eprintln!("Waypie: Invalid activate action format: {}", action_clone);
                            std::process::exit(0); 
                        }
                    });
                } else if action_clone.starts_with("context|") {
                    let parts: Vec<&str> = action_clone.splitn(4, '|').collect();
                    if parts.len() >= 4 {
                        let service = parts[1].to_string();
                        let menu_path = parts[3].to_string();
                        crate::dbus_menu::show_menu(service, menu_path, self.upcast_ref::<gtk4::Widget>(), x, y);
                    }
                } else {
                    if let Err(e) = crate::utils::spawn_app(&action_clone) {
                        eprintln!(
                            "Waypie: Failed to execute command '{}': {}",
                            action_clone, e
                        );
                    } else {
                        std::process::exit(0);
                    }
                }
            }
        }
    }

    fn cartesian_to_polar(&self, x: f64, y: f64, cx: f64, cy: f64) -> (f64, f64) {
        let dx = x - cx;
        let dy = y - cy;
        let dist = (dx * dx + dy * dy).sqrt();
        let theta_rad = dy.atan2(dx);
        let theta_deg = theta_rad.to_degrees();
        (dist, theta_deg)
    }
}
