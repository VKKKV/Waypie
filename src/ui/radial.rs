use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::GestureClick;

use crate::ui::click_logic::resolve_child_click_action;
use crate::ui::hover_state::{
    calculate_hovered_item, get_child_count, get_hover_zone, normalize_angle, HoverZone,
};
pub use crate::ui::menu_model::{Action, PieItem};

use super::radial_imp;

const HOVER_ACTIVATION_DELAY_MS: u64 = 100;
const ANIMATION_TICK_MS: u64 = 16;
const ANIMATION_EPSILON: f64 = 0.001;
const ANIMATION_LERP_SPEED: f64 = 0.2;

glib::wrapper! {
    pub struct RadialMenu(ObjectSubclass<radial_imp::RadialMenu>)
        @extends gtk4::Widget;
}

impl RadialMenu {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_items(&self, items: Vec<PieItem>) {
        let imp = self.imp();
        imp.items.replace(items);
        imp.parent_text_extents.borrow_mut().clear();
        imp.child_text_extents.borrow_mut().clear();
        self.queue_draw();
    }

    pub fn set_ui_config(&self, config: crate::config::UiConfig) {
        self.imp().ui_config.replace(config);
        self.queue_draw();
    }

    pub fn items_equal(&self, other: &[PieItem]) -> bool {
        self.imp().items.borrow().as_slice() == other
    }

    pub fn ui_config_equal(&self, other: &crate::config::UiConfig) -> bool {
        *self.imp().ui_config.borrow() == *other
    }

    pub fn handle_motion(&self, x: f64, y: f64) {
        let imp = self.imp();
        let (cx, cy) = self.widget_center();

        let (dist, angle_deg) = crate::utils::cartesian_to_polar(x, y, cx, cy);
        let items = imp.items.borrow();
        let parent_count = items.len();

        if parent_count == 0 {
            return;
        }

        let mut should_redraw = false;
        let mut keep_hover_child = false;
        let ui = imp.ui_config.borrow();
        let norm_angle = normalize_angle(angle_deg);

        match get_hover_zone(dist, ui.center_radius, ui.inner_radius, ui.outer_radius) {
            HoverZone::Center => {
                should_redraw |= self.clear_hover_parent();
            }
            HoverZone::InnerRing => {
                if let Some(idx) = calculate_hovered_item(norm_angle, parent_count) {
                    should_redraw |= self.set_hover_parent(idx);
                }
            }
            HoverZone::OuterRing => {
                let (outer_redraw, keep_child) = self.update_outer_ring_hover(norm_angle, &items);
                should_redraw |= outer_redraw;
                keep_hover_child = keep_child;
            }
            HoverZone::Outside => {
                should_redraw |= self.clear_hover_parent();
            }
        }

        if !keep_hover_child {
            should_redraw |= self.clear_hover_child();
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

    fn clear_hover_parent(&self) -> bool {
        let imp = self.imp();
        if imp.hover_parent_idx.get().is_some() {
            imp.hover_parent_idx.set(None);
            self.clear_hover_timeout();
            true
        } else {
            false
        }
    }

    fn set_hover_parent(&self, idx: usize) -> bool {
        let imp = self.imp();
        if imp.hover_parent_idx.get() != Some(idx) {
            imp.hover_parent_idx.set(Some(idx));
            self.schedule_hover_activation(idx);
            true
        } else {
            false
        }
    }

    fn clear_hover_child(&self) -> bool {
        let imp = self.imp();
        if imp.hover_child_idx.get().is_some() {
            imp.hover_child_idx.set(None);
            true
        } else {
            false
        }
    }

    fn set_hover_child(&self, idx: usize) -> bool {
        let imp = self.imp();
        if imp.hover_child_idx.get() != Some(idx) {
            imp.hover_child_idx.set(Some(idx));
            true
        } else {
            false
        }
    }

    fn update_outer_ring_hover(&self, norm_angle: f64, items: &[PieItem]) -> (bool, bool) {
        let imp = self.imp();

        if let Some(active_idx) = imp.active_parent_idx.get() {
            let child_count = get_child_count(items, active_idx);
            if child_count > 0 {
                if let Some(idx) = calculate_hovered_item(norm_angle, child_count) {
                    return (self.set_hover_child(idx), true);
                }
            }
            return (false, false);
        }

        (self.clear_hover_parent(), false)
    }

    fn schedule_hover_activation(&self, hover_idx: usize) {
        self.clear_hover_timeout();

        let weak_self = self.downgrade();
        let source_id = gtk4::glib::timeout_add_local(
            std::time::Duration::from_millis(HOVER_ACTIVATION_DELAY_MS),
            move || {
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
            },
        );

        self.imp().hover_timeout_id.replace(Some(source_id));
    }

    fn start_animation(&self, target: f64) {
        let imp = self.imp();
        imp.target_progress.set(target);

        if imp.animation_timeout_id.borrow().is_some() {
            return;
        }

        let current = imp.outer_ring_progress.get();
        if (target - current).abs() < ANIMATION_EPSILON {
            if current != target {
                imp.outer_ring_progress.set(target);
                self.queue_draw();
            }
            return;
        }

        let weak_self = self.downgrade();
        let source_id = gtk4::glib::timeout_add_local(
            std::time::Duration::from_millis(ANIMATION_TICK_MS),
            move || {
                if let Some(menu) = weak_self.upgrade() {
                    let imp = menu.imp();
                    let current = imp.outer_ring_progress.get();
                    let target = imp.target_progress.get();

                    if (target - current).abs() < ANIMATION_EPSILON {
                        if current != target {
                            imp.outer_ring_progress.set(target);
                            menu.queue_draw();
                        }
                        imp.animation_timeout_id.borrow_mut().take();
                        return gtk4::glib::ControlFlow::Break;
                    }

                    let next = current + (target - current) * ANIMATION_LERP_SPEED;
                    imp.outer_ring_progress.set(next);
                    menu.queue_draw();

                    return gtk4::glib::ControlFlow::Continue;
                }

                gtk4::glib::ControlFlow::Break
            },
        );

        imp.animation_timeout_id.replace(Some(source_id));
    }

    fn resolve_clicked_action(&self, button: u32) -> Option<Action> {
        let imp = self.imp();
        let items = imp.items.borrow();

        if let Some(child_idx) = imp.hover_child_idx.get() {
            if let Some(active_idx) = imp.active_parent_idx.get() {
                if let Some(parent) = items.get(active_idx) {
                    if let Some(child) = parent.children.get(child_idx) {
                        return Some(resolve_child_click_action(child, button));
                    }
                }
            }
        }

        if let Some(parent_idx) = imp.hover_parent_idx.get() {
            if let Some(parent) = items.get(parent_idx) {
                if parent.children.is_empty() {
                    return Some(parent.action.clone());
                }
            }
        }

        None
    }

    fn dispatch_action(&self, action: Action, x: f64, y: f64) {
        match action {
            Action::Activate {
                service,
                path,
                menu_path,
            } => self.dispatch_activate(service, path, menu_path, x, y),
            Action::Context {
                service, menu_path, ..
            } => self.dispatch_context(service, menu_path),
            Action::DbusSignal { service, path, id } => {
                self.dispatch_dbus_signal(service, path, id)
            }
            Action::Command(cmd) => self.dispatch_command(cmd),
            Action::None => {}
        }
    }

    fn dispatch_activate(&self, service: String, path: String, menu_path: String, x: f64, y: f64) {
        gtk4::glib::spawn_future_local(async move {
            let success = crate::tray::activate_or_popup(service, path, menu_path, x, y).await;

            if success {
                std::process::exit(0);
            }
        });
    }

    fn dispatch_context(&self, service: String, menu_path: String) {
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

    fn dispatch_dbus_signal(&self, service: String, path: String, id: i32) {
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
                                &(id, "clicked", zbus::zvariant::Value::Str("".into()), 0u32),
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

    fn dispatch_command(&self, cmd: String) {
        if !cmd.is_empty() {
            if let Err(e) = crate::utils::spawn_app(&cmd) {
                eprintln!("Waypie: Failed to execute command '{}': {}", cmd, e);
            } else {
                std::process::exit(0);
            }
        }
    }

    pub fn handle_click(&self, gesture: &GestureClick, _n_press: i32, x: f64, y: f64) {
        let imp = self.imp();
        let (cx, cy) = self.widget_center();

        let (dist, _) = crate::utils::cartesian_to_polar(x, y, cx, cy);
        let button = gesture.current_button();

        let ui = imp.ui_config.borrow();
        if dist < ui.center_radius {
            std::process::exit(0);
        }

        if let Some(action) = self.resolve_clicked_action(button) {
            self.dispatch_action(action, x, y);
        }
    }

    fn widget_center(&self) -> (f64, f64) {
        (self.width() as f64 / 2.0, self.height() as f64 / 2.0)
    }
}
