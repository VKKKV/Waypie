use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::GestureClick;

use crate::ui::click_logic::resolve_clicked_action;
use crate::ui::hover_state::{
    compute_hover_transition, get_child_count, get_hover_zone, normalize_angle,
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

        let ui = imp.ui_config.borrow();
        let norm_angle = normalize_angle(angle_deg);
        let active_parent_idx = imp.active_parent_idx.get();
        let active_child_count = active_parent_idx
            .map(|idx| get_child_count(&items, idx))
            .unwrap_or(0);

        let transition = compute_hover_transition(
            get_hover_zone(dist, ui.center_radius, ui.inner_radius, ui.outer_radius),
            norm_angle,
            parent_count,
            active_parent_idx,
            imp.hover_parent_idx.get(),
            imp.hover_child_idx.get(),
            active_child_count,
        );

        let mut should_redraw = false;

        if transition.clear_hover_timeout {
            self.clear_hover_timeout();
        }
        if imp.hover_parent_idx.get() != transition.next_hover_parent_idx {
            imp.hover_parent_idx.set(transition.next_hover_parent_idx);
            should_redraw = true;
        }
        if imp.hover_child_idx.get() != transition.next_hover_child_idx {
            imp.hover_child_idx.set(transition.next_hover_child_idx);
            should_redraw = true;
        }
        if let Some(idx) = transition.schedule_hover_activation_idx {
            self.schedule_hover_activation(idx);
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

        resolve_clicked_action(
            &items,
            imp.hover_child_idx.get(),
            imp.active_parent_idx.get(),
            imp.hover_parent_idx.get(),
            button,
        )
    }

    fn dispatch_action(&self, action: Action, x: f64, y: f64) {
        crate::ui::action_dispatcher::dispatch_action(self, action, x, y);
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
