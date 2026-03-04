use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{EventControllerMotion, GestureClick, Snapshot};
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;

use super::radial::PieItem;

#[derive(Default)]
pub struct RadialMenu {
    // Data
    pub items: RefCell<Vec<PieItem>>,
    pub ui_config: RefCell<crate::config::UiConfig>,

    // State
    pub active_parent_idx: Cell<Option<usize>>,
    pub hover_parent_idx: Cell<Option<usize>>,
    pub hover_child_idx: Cell<Option<usize>>,
    pub hover_timeout_id: RefCell<Option<glib::SourceId>>,

    // Animation
    pub outer_ring_progress: Cell<f64>, // 0.0 to 1.0
    pub target_progress: Cell<f64>,     // 0.0 or 1.0
    pub animation_timeout_id: RefCell<Option<glib::SourceId>>,
}

#[glib::object_subclass]
impl ObjectSubclass for RadialMenu {
    const NAME: &'static str = "RadialMenu";
    type Type = super::radial::RadialMenu;
    type ParentType = gtk4::Widget;
}

impl ObjectImpl for RadialMenu {
    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();

        // Clock Timer
        let weak_obj = obj.downgrade();
        gtk4::glib::timeout_add_local(std::time::Duration::from_secs(1), move || {
            if let Some(obj) = weak_obj.upgrade() {
                obj.queue_draw();
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });

        // Motion Controller
        let motion = EventControllerMotion::new();
        let weak_obj = obj.downgrade();
        motion.connect_motion(move |_, x, y| {
            if let Some(obj) = weak_obj.upgrade() {
                obj.handle_motion(x, y);
            }
        });
        let weak_obj = obj.downgrade();
        motion.connect_leave(move |_| {
            if let Some(obj) = weak_obj.upgrade() {
                obj.handle_leave();
            }
        });
        obj.add_controller(motion);

        // Click Controller
        let click = GestureClick::new();
        click.set_button(0);
        let weak_obj = obj.downgrade();
        click.connect_pressed(move |gesture, n_press, x, y| {
            if let Some(obj) = weak_obj.upgrade() {
                obj.handle_click(gesture, n_press, x, y);
            }
        });
        obj.add_controller(click);
    }
}

impl WidgetImpl for RadialMenu {
    fn snapshot(&self, snapshot: &Snapshot) {
        let obj = self.obj();
        let w = obj.width() as f64;
        let h = obj.height() as f64;

        let cr = snapshot.append_cairo(&gtk4::graphene::Rect::new(0.0, 0.0, w as f32, h as f32));

        let center_x = w / 2.0;
        let center_y = h / 2.0;

        let items = self.items.borrow();
        let parent_count = items.len();
        let ui = self.ui_config.borrow();

        // --- Draw Center ---
        let center_radius = ui.center_radius;

        // Background for center
        let (r, g, b, a) = ui.colors.center_color;
        cr.set_source_rgba(r, g, b, a);
        cr.arc(center_x, center_y, center_radius, 0.0, 2.0 * PI);
        cr.fill().unwrap();

        // Time
        let now = chrono::Local::now();
        let time_str = now.format("%H:%M").to_string();
        let (tr, tg, tb) = ui.colors.text_color;
        cr.set_source_rgb(tr, tg, tb);
        cr.set_font_size(20.0);
        let ext = cr.text_extents(&time_str).unwrap();
        cr.move_to(center_x - ext.width() / 2.0, center_y + ext.height() / 4.0);
        cr.show_text(&time_str).unwrap();

        if parent_count == 0 {
            return;
        }

        // --- Draw Inner Ring (Parents) ---
        let inner_radius_start = center_radius;
        let inner_radius_end = ui.inner_radius;
        let angle_per_parent = 2.0 * PI / parent_count as f64;
        let start_offset = -PI / 2.0; // 12 o'clock

        for (i, item) in items.iter().enumerate() {
            let start_angle = start_offset + (i as f64 * angle_per_parent);
            let end_angle = start_angle + angle_per_parent;

            // Determine Color
            let (r, g, b, a) = if Some(i) == self.hover_parent_idx.get() {
                ui.colors.inner_ring_color_hover
            } else if Some(i) == self.active_parent_idx.get() {
                ui.colors.inner_ring_color_active
            } else if i % 2 == 0 {
                ui.colors.inner_ring_color_even
            } else {
                ui.colors.inner_ring_color_odd
            };
            cr.set_source_rgba(r, g, b, a);

            // Draw Segment
            cr.new_path();
            cr.arc(center_x, center_y, inner_radius_end, start_angle, end_angle);
            cr.arc_negative(
                center_x,
                center_y,
                inner_radius_start,
                end_angle,
                start_angle,
            );
            cr.close_path();
            cr.fill().unwrap();

            // Stroke
            let (sr, sg, sb) = ui.colors.stroke_color;
            cr.set_source_rgb(sr, sg, sb);
            cr.set_line_width(1.0);
            cr.stroke().unwrap();

            // Text
            cr.set_source_rgb(tr, tg, tb);
            cr.set_font_size(12.0);
            let text_radius = (inner_radius_start + inner_radius_end) / 2.0;
            let mid_angle = start_angle + angle_per_parent / 2.0;
            let tx = center_x + text_radius * mid_angle.cos();
            let ty = center_y + text_radius * mid_angle.sin();

            let ext = cr.text_extents(&item.label).unwrap();
            cr.move_to(tx - ext.width() / 2.0, ty + ext.height() / 4.0);
            cr.show_text(&item.label).unwrap();
        }

        // --- Draw Outer Ring (Children) ---
        // Use animation progress
        let progress = self.outer_ring_progress.get();

        if progress > 0.01 {
            if let Some(active_idx) = self.active_parent_idx.get() {
                if let Some(parent) = items.get(active_idx) {
                    // Determine Children Source
                    let children = &parent.children;
                    let child_count = children.len();

                    if child_count > 0 {
                        // "Slide out" effect based on configured radii
                        let base_start = ui.inner_radius - 20.0;
                        let target_start = ui.inner_radius + 10.0;
                        let base_end = ui.outer_radius - 50.0;
                        let target_end = ui.outer_radius;

                        let outer_radius_start =
                            base_start + (target_start - base_start) * progress;
                        let outer_radius_end = base_end + (target_end - base_end) * progress;

                        let angle_per_child = 2.0 * PI / child_count as f64;

                        // Alpha multiplier
                        let alpha = progress;

                        for (j, child) in children.iter().enumerate() {
                            let start_angle = start_offset + (j as f64 * angle_per_child);
                            let end_angle = start_angle + angle_per_child;

                            // Color
                            let (r, g, b, a) = if Some(j) == self.hover_child_idx.get() {
                                ui.colors.outer_ring_color_hover
                            } else if j % 2 == 0 {
                                ui.colors.outer_ring_color_even
                            } else {
                                ui.colors.outer_ring_color_odd
                            };
                            cr.set_source_rgba(r, g, b, a * alpha);

                            cr.new_path();
                            cr.arc(center_x, center_y, outer_radius_end, start_angle, end_angle);
                            cr.arc_negative(
                                center_x,
                                center_y,
                                outer_radius_start,
                                end_angle,
                                start_angle,
                            );
                            cr.close_path();
                            cr.fill().unwrap();

                            // Stroke
                            let (sr, sg, sb) = ui.colors.stroke_color;
                            cr.set_source_rgba(sr, sg, sb, alpha);
                            cr.set_line_width(1.0);
                            cr.stroke().unwrap();

                            // Text
                            let (tr, tg, tb) = ui.colors.text_color;
                            cr.set_source_rgba(tr, tg, tb, alpha);
                            cr.set_font_size(11.0);
                            let text_radius = (outer_radius_start + outer_radius_end) / 2.0;
                            let mid_angle = start_angle + angle_per_child / 2.0;
                            let tx = center_x + text_radius * mid_angle.cos();
                            let ty = center_y + text_radius * mid_angle.sin();

                            let ext = cr.text_extents(&child.label).unwrap();
                            cr.move_to(tx - ext.width() / 2.0, ty + ext.height() / 4.0);
                            cr.show_text(&child.label).unwrap();
                        }
                    }
                }
            }
        }
    }
}
