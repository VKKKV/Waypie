use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::GestureClick;
use std::time::{Duration, Instant};

// 1. Data Structure
#[derive(Clone, Debug)]
pub enum Action {
    Command(String),
    Activate {
        service: String,
        path: String,
        menu_path: String,
    },
    Context {
        service: String,
        path: String,
        menu_path: String,
    },
    DbusSignal {
        service: String,
        path: String,
        id: i32,
    },
    None,
}

impl Action {
    pub fn from_string(s: String) -> Self {
        if s.is_empty() {
            return Action::None;
        }

        if let Some(rest) = s.strip_prefix("activate|") {
            let parts: Vec<&str> = rest.splitn(3, '|').collect();
            if parts.len() == 3 {
                return Action::Activate {
                    service: parts[0].to_string(),
                    path: parts[1].to_string(),
                    menu_path: parts[2].to_string(),
                };
            }
        } else if let Some(rest) = s.strip_prefix("context|") {
            let parts: Vec<&str> = rest.splitn(3, '|').collect();
            if parts.len() == 3 {
                return Action::Context {
                    service: parts[0].to_string(),
                    path: parts[1].to_string(),
                    menu_path: parts[2].to_string(),
                };
            }
        } else if let Some(rest) = s.strip_prefix("dbus_signal|") {
            let parts: Vec<&str> = rest.splitn(3, '|').collect();
            if parts.len() == 3 {
                if let Ok(id) = parts[2].parse::<i32>() {
                    return Action::DbusSignal {
                        service: parts[0].to_string(),
                        path: parts[1].to_string(),
                        id,
                    };
                }
            }
        }

        Action::Command(s)
    }

    pub fn to_string(&self) -> String {
        match self {
            Action::Command(cmd) => cmd.clone(),
            Action::Activate {
                service,
                path,
                menu_path,
            } => format!("activate|{}|{}|{}", service, path, menu_path),
            Action::Context {
                service,
                path,
                menu_path,
            } => format!("context|{}|{}|{}", service, path, menu_path),
            Action::DbusSignal { service, path, id } => {
                format!("dbus_signal|{}|{}|{}", service, path, id)
            }
            Action::None => String::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PieItem {
    pub label: String,
    pub icon: String,
    pub action: Action,
    pub children: Vec<PieItem>,
    pub item_type: Option<String>,
    pub tray_id: Option<String>,
}

use super::radial_imp;

glib::wrapper! {
    pub struct RadialMenu(ObjectSubclass<radial_imp::RadialMenu>)
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

    pub fn set_ui_config(&self, config: crate::config::UiConfig) {
        self.imp().ui_config.replace(config);
        self.queue_draw();
    }

    fn normalize_angle(angle: f64) -> f64 {
        let mut normalized = angle + 90.0;
        if normalized < 0.0 {
            normalized += 360.0;
        }
        normalized
    }

    fn calculate_hovered_item(angle: f64, item_count: usize) -> Option<usize> {
        if item_count == 0 {
            return None;
        }
        let angle_per_item = 360.0 / item_count as f64;
        let idx = (angle / angle_per_item).floor() as usize;
        Some(idx.min(item_count - 1))
    }

    pub fn handle_motion(&self, x: f64, y: f64) {
        let imp = self.imp();
        let w = self.width() as f64;
        let h = self.height() as f64;
        let cx = w / 2.0;
        let cy = h / 2.0;

        let (dist, angle_deg) = crate::utils::cartesian_to_polar(x, y, cx, cy);
        let items = imp.items.borrow();
        let parent_count = items.len();

        if parent_count == 0 {
            return;
        }

        let mut should_redraw = false;
        let mut reset_hover_child = true;
        let ui = imp.ui_config.borrow();
        let norm_angle = Self::normalize_angle(angle_deg);

        if dist <= ui.center_radius {
            if imp.hover_parent_idx.get().is_some() {
                imp.hover_parent_idx.set(None);
                imp.hover_start_time.set(None);
                should_redraw = true;
            }
        } else if dist <= ui.inner_radius {
            if let Some(idx) = Self::calculate_hovered_item(norm_angle, parent_count) {
                if imp.hover_parent_idx.get() != Some(idx) {
                    imp.hover_parent_idx.set(Some(idx));
                    imp.hover_start_time.set(Some(Instant::now()));
                    should_redraw = true;
                }
            }
        } else if dist >= ui.inner_radius + 10.0 && dist <= ui.outer_radius {
            if let Some(active_idx) = imp.active_parent_idx.get() {
                let child_count = items.get(active_idx).map(|p| p.children.len()).unwrap_or(0);
                if child_count > 0 {
                    if let Some(idx) = Self::calculate_hovered_item(norm_angle, child_count) {
                        if imp.hover_child_idx.get() != Some(idx) {
                            imp.hover_child_idx.set(Some(idx));
                            should_redraw = true;
                        }
                        reset_hover_child = false;
                    }
                }
            } else if imp.hover_parent_idx.get().is_some() {
                imp.hover_parent_idx.set(None);
                imp.hover_start_time.set(None);
                should_redraw = true;
            }
        } else if imp.hover_parent_idx.get().is_some() {
            imp.hover_parent_idx.set(None);
            imp.hover_start_time.set(None);
            should_redraw = true;
        }

        if reset_hover_child && imp.hover_child_idx.get().is_some() {
            imp.hover_child_idx.set(None);
            should_redraw = true;
        }

        if should_redraw {
            self.queue_draw();
        }
    }

    pub fn handle_leave(&self) {
        let imp = self.imp();
        imp.hover_parent_idx.set(None);
        imp.hover_child_idx.set(None);

        // Don't reset active parent immediately on leave if you want it to stay open?
        // Prompt says "If mouse leaves logic area: Reset state."
        imp.active_parent_idx.set(None);
        imp.target_progress.set(0.0); // Animate out

        self.queue_draw();
    }

    pub fn check_hover_timer(&self) {
        let imp = self.imp();

        if let Some(hover_idx) = imp.hover_parent_idx.get() {
            if Some(hover_idx) != imp.active_parent_idx.get() {
                if let Some(start_time) = imp.hover_start_time.get() {
                    if start_time.elapsed() >= Duration::from_millis(100) {
                        imp.active_parent_idx.set(Some(hover_idx));

                        let child_count = imp
                            .items
                            .borrow()
                            .get(hover_idx)
                            .map(|p| p.children.len())
                            .unwrap_or(0);
                        let target = if child_count > 0 { 1.0 } else { 0.0 };

                        if (imp.target_progress.get() - target).abs() > 0.001 {
                            imp.target_progress.set(target);
                            self.queue_draw();
                        } else {
                            self.queue_draw();
                        }
                    }
                }
            }
        }
    }

    pub fn handle_click(&self, gesture: &GestureClick, _n_press: i32, x: f64, y: f64) {
        let imp = self.imp();
        let w = self.width() as f64;
        let h = self.height() as f64;
        let cx = w / 2.0;
        let cy = h / 2.0;

        let (dist, _angle_deg) = crate::utils::cartesian_to_polar(x, y, cx, cy);
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
                            && button == gtk4::gdk::BUTTON_PRIMARY || button == gtk4::gdk::BUTTON_SECONDARY
                        {
                            if let Action::Activate {
                                service,
                                path,
                                menu_path,
                            } = &child.action
                            {
                                clicked_action = Some(Action::Context {
                                    service: service.clone(),
                                    path: path.clone(),
                                    menu_path: menu_path.clone(),
                                });
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
            match action {
                Action::Activate {
                    service,
                    path,
                    menu_path,
                } => {
                    let self_clone = self.clone();
                    gtk4::glib::spawn_future_local(async move {
                        let success = crate::tray::activate_or_popup(
                            service,
                            path,
                            menu_path,
                            self_clone.upcast_ref::<gtk4::Widget>().clone(),
                            x,
                            y,
                        )
                        .await;

                        if success {
                            std::process::exit(0);
                        }
                    });
                }
                Action::Context {
                    service, menu_path, ..
                } => {
                    let self_clone = self.clone();
                    gtk4::glib::spawn_future_local(async move {
                        match crate::tray::fetch_dbus_menu_as_pie(service, menu_path).await {
                            Ok(items) => {
                                println!("Waypie: Context menu fetched with {} items", items.len());
                                self_clone.set_items(items);
                            }
                            Err(e) => eprintln!("Waypie: Failed to fetch context menu: {}", e),
                        }
                    });
                }
                Action::DbusSignal { service, path, id } => {
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
                                            &(
                                                id,
                                                "clicked",
                                                zbus::zvariant::Value::Str("".into()),
                                                0u32,
                                            ),
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
                Action::Command(cmd) => {
                    if !cmd.is_empty() {
                        if let Err(e) = crate::utils::spawn_app(&cmd) {
                            eprintln!("Waypie: Failed to execute command '{}': {}", cmd, e);
                        } else {
                            std::process::exit(0);
                        }
                    }
                }
                Action::None => {}
            }
        }
    }
}
