use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::GestureClick;

use crate::ui::hover_state::{
    calculate_hovered_item, get_child_count, get_hover_zone, normalize_angle, HoverZone,
};
pub use crate::ui::menu_model::{Action, PieItem};

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
        let norm_angle = normalize_angle(angle_deg);

        match get_hover_zone(dist, ui.center_radius, ui.inner_radius, ui.outer_radius) {
            HoverZone::Center => {
                if imp.hover_parent_idx.get().is_some() {
                    imp.hover_parent_idx.set(None);
                    self.clear_hover_timeout();
                    should_redraw = true;
                }
            }
            HoverZone::InnerRing => {
                if let Some(idx) = calculate_hovered_item(norm_angle, parent_count) {
                    if imp.hover_parent_idx.get() != Some(idx) {
                        imp.hover_parent_idx.set(Some(idx));
                        self.schedule_hover_activation(idx);
                        should_redraw = true;
                    }
                }
            }
            HoverZone::OuterRing => {
                if let Some(active_idx) = imp.active_parent_idx.get() {
                    let child_count = get_child_count(&items, active_idx);
                    if child_count > 0 {
                        if let Some(idx) = calculate_hovered_item(norm_angle, child_count) {
                            if imp.hover_child_idx.get() != Some(idx) {
                                imp.hover_child_idx.set(Some(idx));
                                should_redraw = true;
                            }
                            reset_hover_child = false;
                        }
                    }
                } else if imp.hover_parent_idx.get().is_some() {
                    imp.hover_parent_idx.set(None);
                    self.clear_hover_timeout();
                    should_redraw = true;
                }
            }
            HoverZone::Outside => {
                if imp.hover_parent_idx.get().is_some() {
                    imp.hover_parent_idx.set(None);
                    self.clear_hover_timeout();
                    should_redraw = true;
                }
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

    pub fn handle_leave(&self) {
        let imp = self.imp();
        self.clear_hover_timeout();
        imp.hover_parent_idx.set(None);
        imp.hover_child_idx.set(None);
        imp.active_parent_idx.set(None);
        self.start_animation(0.0);
        self.queue_draw();
    }

    fn clear_hover_timeout(&self) {
        if let Some(source_id) = self.imp().hover_timeout_id.borrow_mut().take() {
            source_id.remove();
        }
    }

    fn schedule_hover_activation(&self, hover_idx: usize) {
        self.clear_hover_timeout();

        let weak_self = self.downgrade();
        let source_id =
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                if let Some(menu) = weak_self.upgrade() {
                    let imp = menu.imp();

                    if imp.hover_parent_idx.get() == Some(hover_idx)
                        && imp.active_parent_idx.get() != Some(hover_idx)
                    {
                        imp.active_parent_idx.set(Some(hover_idx));
                        let child_count = get_child_count(&imp.items.borrow(), hover_idx);
                        let target = if child_count > 0 { 1.0 } else { 0.0 };
                        menu.start_animation(target);
                        menu.queue_draw();
                    }

                    imp.hover_timeout_id.borrow_mut().take();
                }

                gtk4::glib::ControlFlow::Break
            });

        self.imp().hover_timeout_id.replace(Some(source_id));
    }

    fn start_animation(&self, target: f64) {
        let imp = self.imp();
        imp.target_progress.set(target);

        if imp.animation_timeout_id.borrow().is_some() {
            return;
        }

        let current = imp.outer_ring_progress.get();
        if (target - current).abs() < 0.001 {
            if current != target {
                imp.outer_ring_progress.set(target);
                self.queue_draw();
            }
            return;
        }

        let weak_self = self.downgrade();
        let source_id =
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
                if let Some(menu) = weak_self.upgrade() {
                    let imp = menu.imp();
                    let current = imp.outer_ring_progress.get();
                    let target = imp.target_progress.get();

                    if (target - current).abs() < 0.001 {
                        if current != target {
                            imp.outer_ring_progress.set(target);
                            menu.queue_draw();
                        }
                        imp.animation_timeout_id.borrow_mut().take();
                        return gtk4::glib::ControlFlow::Break;
                    }

                    let next = current + (target - current) * 0.2;
                    imp.outer_ring_progress.set(next);
                    menu.queue_draw();

                    return gtk4::glib::ControlFlow::Continue;
                }

                gtk4::glib::ControlFlow::Break
            });

        imp.animation_timeout_id.replace(Some(source_id));
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

        let ui = imp.ui_config.borrow();
        if dist < ui.center_radius {
            std::process::exit(0);
        }

        let mut clicked_action = None;

        if let Some(child_idx) = imp.hover_child_idx.get() {
            if let Some(active_idx) = imp.active_parent_idx.get() {
                if let Some(parent) = items.get(active_idx) {
                    if let Some(child) = parent.children.get(child_idx) {
                        if child.item_type.as_deref() == Some("tray_app")
                            && button == gtk4::gdk::BUTTON_SECONDARY
                            || button == gtk4::gdk::BUTTON_PRIMARY
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
