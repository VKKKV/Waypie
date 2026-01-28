use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, DrawingArea, GestureClick, EventControllerScroll, EventControllerScrollFlags, EventControllerMotion};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use gtk4::glib;
use chrono::Local;
use std::f64::consts::PI;
use std::time::Duration;
use std::process::Command;
use std::rc::Rc;
use std::cell::RefCell;
use crate::config::AppConfig;
use crate::utils::execute_command;

#[derive(Clone, Copy, PartialEq)]
enum HoverTarget {
    None,
    Center,
    RingItem(usize),
    OuterRingItem(usize),
}

pub fn run(app_id: &str, config: AppConfig) {
    let app = Application::builder()
        .application_id(app_id)
        .build();

    app.connect_activate(move |app| build_ui(app, config.clone()));
    app.run_with_args(&Vec::<String>::new());
}

fn build_ui(app: &Application, config: AppConfig) {
    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(config.ui.width)
        .default_height(config.ui.height)
        .build();

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
    // Remove anchors to allow the window to float/center with default_width/height
    window.set_anchor(Edge::Left, false);
    window.set_anchor(Edge::Right, false);
    window.set_anchor(Edge::Top, false);
    window.set_anchor(Edge::Bottom, false);
    window.set_css_classes(&["transparent"]);

    // Exit on ESC
    let controller = gtk4::EventControllerKey::new();
    controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Escape {
            std::process::exit(0);
        }
        gtk4::glib::Propagation::Stop
    });
    window.add_controller(controller);

    let drawing_area = DrawingArea::new();
    drawing_area.set_focusable(true);
    let config_rc = Rc::new(config);
    let hover_state = Rc::new(RefCell::new(HoverTarget::None));
    let active_submenu = Rc::new(RefCell::new(Option::<usize>::None));
    
    // Configurable refresh rate
    let da_weak = drawing_area.downgrade();
    let refresh_ms = config_rc.ui.refresh_rate_ms;
    glib::timeout_add_local(Duration::from_millis(refresh_ms), move || {
        if let Some(da) = da_weak.upgrade() {
            da.queue_draw();
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });

    let draw_config = config_rc.clone();
    let draw_hover = hover_state.clone();
    let draw_active = active_submenu.clone();
    drawing_area.set_draw_func(move |_, context, width, height| {
        let w = width as f64;
        let h = height as f64;
        let center_x = w / 2.0;
        let center_y = h / 2.0;

        let now = Local::now();
        let volume = get_volume(); 
        let current_hover = *draw_hover.borrow();
        let current_active = *draw_active.borrow();

        draw_radial_wheel(context, center_x, center_y, &draw_config, now, volume, current_hover, current_active);
    });

    // Interaction (Click)
    setup_click_handler(&drawing_area, config_rc.clone(), active_submenu.clone());
    // Interaction (Scroll)
    setup_scroll_handler(&drawing_area, config_rc.clone());
    // Interaction (Hover)
    setup_hover_handler(&drawing_area, config_rc.clone(), hover_state, active_submenu);

    window.set_child(Some(&drawing_area));
    window.present();
}

fn draw_radial_wheel(
    context: &gtk4::cairo::Context, 
    cx: f64, 
    cy: f64, 
    config: &AppConfig, 
    now: chrono::DateTime<Local>, 
    volume: f64,
    hover: HoverTarget,
    active_submenu: Option<usize>,
) {
    let ui = &config.ui;
    let colors = &ui.colors;

    // 1. Background
    context.set_source_rgba(colors.background.0, colors.background.1, colors.background.2, colors.background.3);
    context.arc(cx, cy, ui.outer_radius, 0.0, 2.0 * PI);
    context.fill().expect("Failed to fill bg");

    // 2. Tray Items (Main Ring)
    let items_count = config.items.len();
    if items_count > 0 {
        let angle_per_item = 2.0 * PI / items_count as f64;
        let start_offset = -PI / 2.0; // Top

        for (i, item) in config.items.iter().enumerate() {
            let start_angle = start_offset + (i as f64 * angle_per_item);
            let end_angle = start_angle + angle_per_item;

            // Segment Colors
            if i % 2 == 0 {
                context.set_source_rgba(colors.tray_even.0, colors.tray_even.1, colors.tray_even.2, colors.tray_even.3);
            } else {
                context.set_source_rgba(colors.tray_odd.0, colors.tray_odd.1, colors.tray_odd.2, colors.tray_odd.3);
            }

            // Draw Segment
            context.new_path();
            context.arc(cx, cy, ui.outer_radius, start_angle, end_angle);
            context.arc_negative(cx, cy, ui.tray_inner_radius, end_angle, start_angle);
            context.close_path();
            context.fill().unwrap();

            // Hover Effect for Main Ring
            if ui.hover_mode == "highlight" {
                if let HoverTarget::RingItem(h_idx) = hover {
                    if h_idx == i {
                        context.set_source_rgba(colors.hover_overlay.0, colors.hover_overlay.1, colors.hover_overlay.2, colors.hover_overlay.3);
                        context.new_path();
                        context.arc(cx, cy, ui.outer_radius, start_angle, end_angle);
                        context.arc_negative(cx, cy, ui.tray_inner_radius, end_angle, start_angle);
                        context.close_path();
                        context.fill().unwrap();
                    }
                }
            }

            // Draw Label
            let mid_angle = start_angle + (angle_per_item / 2.0);
            let text_radius = (ui.outer_radius + ui.tray_inner_radius) / 2.0;
            let tx = cx + text_radius * mid_angle.cos();
            let ty = cy + text_radius * mid_angle.sin();

            context.set_source_rgb(colors.text.0, colors.text.1, colors.text.2);
            context.select_font_face(&ui.font_family, gtk4::cairo::FontSlant::Normal, gtk4::cairo::FontWeight::Bold);
            context.set_font_size(13.0);
            let extents = context.text_extents(&item.label).unwrap();
            context.move_to(tx - extents.width() / 2.0, ty + extents.height() / 4.0);
            context.show_text(&item.label).unwrap();
            
            // Separator
            context.set_source_rgb(0.0, 0.0, 0.0);
            context.set_line_width(2.0);
            context.move_to(cx + ui.tray_inner_radius * start_angle.cos(), cy + ui.tray_inner_radius * start_angle.sin());
            context.line_to(cx + ui.outer_radius * start_angle.cos(), cy + ui.outer_radius * start_angle.sin());
            context.stroke().unwrap();
        }
    }

    // 3. Outer Ring (Submenu)
    if let Some(parent_idx) = active_submenu {
        if let Some(parent) = config.items.get(parent_idx) {
            let sub_items = &parent.items;
            let sub_count = sub_items.len();
            if sub_count > 0 {
                let sub_outer_radius = ui.outer_radius + 80.0;
                let sub_inner_radius = ui.outer_radius + 5.0; // Small gap

                let angle_per_sub = 2.0 * PI / sub_count as f64;
                let start_offset = -PI / 2.0;

                for (i, item) in sub_items.iter().enumerate() {
                    let start_angle = start_offset + (i as f64 * angle_per_sub);
                    let end_angle = start_angle + angle_per_sub;

                    // Reuse colors but slightly different
                    if i % 2 == 0 {
                         context.set_source_rgba(colors.tray_even.0, colors.tray_even.1, colors.tray_even.2, colors.tray_even.3);
                    } else {
                         context.set_source_rgba(colors.tray_odd.0, colors.tray_odd.1, colors.tray_odd.2, colors.tray_odd.3);
                    }

                    context.new_path();
                    context.arc(cx, cy, sub_outer_radius, start_angle, end_angle);
                    context.arc_negative(cx, cy, sub_inner_radius, end_angle, start_angle);
                    context.close_path();
                    context.fill().unwrap();

                    // Hover Effect for Outer Ring
                    if ui.hover_mode == "highlight" {
                        if let HoverTarget::OuterRingItem(h_idx) = hover {
                            if h_idx == i {
                                context.set_source_rgba(colors.hover_overlay.0, colors.hover_overlay.1, colors.hover_overlay.2, colors.hover_overlay.3);
                                context.new_path();
                                context.arc(cx, cy, sub_outer_radius, start_angle, end_angle);
                                context.arc_negative(cx, cy, sub_inner_radius, end_angle, start_angle);
                                context.close_path();
                                context.fill().unwrap();
                            }
                        }
                    }

                     // Label
                    let mid_angle = start_angle + (angle_per_sub / 2.0);
                    let text_radius = (sub_outer_radius + sub_inner_radius) / 2.0;
                    let tx = cx + text_radius * mid_angle.cos();
                    let ty = cy + text_radius * mid_angle.sin();

                    context.set_source_rgb(colors.text.0, colors.text.1, colors.text.2);
                    context.select_font_face(&ui.font_family, gtk4::cairo::FontSlant::Normal, gtk4::cairo::FontWeight::Bold);
                    context.set_font_size(12.0);
                    let extents = context.text_extents(&item.label).unwrap();
                    context.move_to(tx - extents.width() / 2.0, ty + extents.height() / 4.0);
                    context.show_text(&item.label).unwrap();

                    // Separator
                    context.set_source_rgb(0.0, 0.0, 0.0);
                    context.set_line_width(2.0);
                    context.move_to(cx + sub_inner_radius * start_angle.cos(), cy + sub_inner_radius * start_angle.sin());
                    context.line_to(cx + sub_outer_radius * start_angle.cos(), cy + sub_outer_radius * start_angle.sin());
                    context.stroke().unwrap();
                }
            }
        }
    }

    // Hover Effect for Center
    if ui.hover_mode == "highlight" && hover == HoverTarget::Center {
        context.set_source_rgba(colors.hover_overlay.0, colors.hover_overlay.1, colors.hover_overlay.2, colors.hover_overlay.3);
        context.new_path();
        context.arc(cx, cy, ui.tray_inner_radius, 0.0, 2.0 * PI);
        context.fill().unwrap();
    }

    // 4. Volume Arc (Inner Ring)
    // Track
    context.set_source_rgba(colors.volume_track.0, colors.volume_track.1, colors.volume_track.2, colors.volume_track.3);
    context.set_line_width(8.0);
    context.set_line_cap(gtk4::cairo::LineCap::Round);
    context.arc(cx, cy, ui.vol_radius, 0.0, 2.0 * PI);
    context.stroke().unwrap();

    // Active Level
    if volume > 0.8 {
        context.set_source_rgb(colors.volume_warning.0, colors.volume_warning.1, colors.volume_warning.2);
    } else {
        context.set_source_rgb(colors.volume_arc.0, colors.volume_arc.1, colors.volume_arc.2);
    }

    let vol_angle = volume * 2.0 * PI;
    let start_angle = -PI / 2.0;

    context.arc(cx, cy, ui.vol_radius, start_angle, start_angle + vol_angle);
    context.stroke().unwrap();

    // 5. Center Info
    context.set_source_rgb(colors.text.0, colors.text.1, colors.text.2);
    
    // Time
    let time_str = now.format("%H:%M").to_string();
    context.set_font_size(42.0);
    let ext_time = context.text_extents(&time_str).unwrap();
    context.move_to(cx - ext_time.width() / 2.0, cy - 10.0);
    context.show_text(&time_str).unwrap();

    // Date
    let date_str = now.format("%a %d %b").to_string();
    context.set_font_size(16.0);
    let ext_date = context.text_extents(&date_str).unwrap();
    context.move_to(cx - ext_date.width() / 2.0, cy + 20.0);
    context.show_text(&date_str).unwrap();

    // Volume Text
    let vol_text = format!("Vol: {:.0}%", volume * 100.0);
    context.set_font_size(14.0);
    let ext_vol = context.text_extents(&vol_text).unwrap();
    context.move_to(cx - ext_vol.width() / 2.0, cy + 45.0);
    context.show_text(&vol_text).unwrap();
}

fn setup_click_handler(
    drawing_area: &DrawingArea, 
    config: Rc<AppConfig>, 
    active_submenu: Rc<RefCell<Option<usize>>>
) {
    let click = GestureClick::new();
    click.set_button(0); 
    
    let click_cfg = config.clone();
    let click_state = active_submenu.clone();
    let widget_weak = drawing_area.downgrade();

    click.connect_pressed(move |gesture, _, x, y| {
        let ui = &click_cfg.ui;
        
        let widget = gesture.widget().unwrap();
        let w = widget.width() as f64;
        let h = widget.height() as f64;
        let center_x = w / 2.0;
        let center_y = h / 2.0;
        
        let dx = x - center_x;
        let dy = y - center_y;
        let dist = (dx * dx + dy * dy).sqrt();
        
        let button = gesture.current_button();
        let mut current_active = click_state.borrow_mut();

        // Radii for outer ring
        let sub_inner = ui.outer_radius + 5.0;
        let sub_outer = ui.outer_radius + 80.0;

        // 1. Outer Ring (Submenu)
        if current_active.is_some() && dist >= sub_inner && dist <= sub_outer {
             if button == gtk4::gdk::BUTTON_PRIMARY {
                if let Some(parent_idx) = *current_active {
                    if let Some(parent) = click_cfg.items.get(parent_idx) {
                        let sub_items = &parent.items;
                        let sub_count = sub_items.len();
                        if sub_count > 0 {
                            let angle = dy.atan2(dx);
                            let mut active_angle = angle + PI / 2.0;
                            if active_angle < 0.0 { active_angle += 2.0 * PI; }
                            
                            let angle_per_item = 2.0 * PI / sub_count as f64;
                            let index = (active_angle / angle_per_item).floor() as usize;
                            
                            if index < sub_count {
                                if let Some(script) = &sub_items[index].script {
                                    execute_command(script);
                                    *current_active = None;
                                    if let Some(da) = widget_weak.upgrade() { da.queue_draw(); }
                                }
                            }
                        }
                    }
                }
             }
        }
        // 2. Main Ring
        else if dist >= ui.tray_inner_radius && dist <= ui.outer_radius {
             if button == gtk4::gdk::BUTTON_PRIMARY {
                let items_count = click_cfg.items.len();
                if items_count > 0 {
                    let angle = dy.atan2(dx);
                    let mut active_angle = angle + PI / 2.0;
                    if active_angle < 0.0 { active_angle += 2.0 * PI; }
                    
                    let angle_per_item = 2.0 * PI / items_count as f64;
                    let index = (active_angle / angle_per_item).floor() as usize;
                    
                    if index < items_count {
                        let item = &click_cfg.items[index];
                        if !item.items.is_empty() {
                            if *current_active == Some(index) {
                                *current_active = None;
                            } else {
                                *current_active = Some(index);
                            }
                            if let Some(da) = widget_weak.upgrade() { da.queue_draw(); }
                        } else {
                            if let Some(script) = &item.script {
                                execute_command(script);
                                *current_active = None;
                                if let Some(da) = widget_weak.upgrade() { da.queue_draw(); }
                            }
                        }
                    }
                }
             }
        } 
        // 3. Central Hub
        else if dist < ui.tray_inner_radius {
            if button == gtk4::gdk::BUTTON_PRIMARY {
                if let Some(cmd) = &click_cfg.actions.left_click {
                    execute_command(cmd);
                }
            } else if button == gtk4::gdk::BUTTON_SECONDARY {
                if let Some(cmd) = &click_cfg.actions.right_click {
                    execute_command(cmd);
                }
            }
        }
    });
    
    drawing_area.add_controller(click);
}

fn setup_scroll_handler(drawing_area: &DrawingArea, config: Rc<AppConfig>) {
    let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
    let scroll_cfg = config.clone();

    scroll.connect_scroll(move |_controller, _dx, dy| {
        if dy < 0.0 {
            if let Some(cmd) = &scroll_cfg.actions.scroll_up {
                execute_command(cmd);
            }
        } else {
            if let Some(cmd) = &scroll_cfg.actions.scroll_down {
                execute_command(cmd);
            }
        }
        gtk4::glib::Propagation::Stop
    });
    
    drawing_area.add_controller(scroll);
}

fn setup_hover_handler(
    drawing_area: &DrawingArea, 
    config: Rc<AppConfig>, 
    hover_state: Rc<RefCell<HoverTarget>>,
    active_submenu: Rc<RefCell<Option<usize>>>
) {
    let motion = EventControllerMotion::new();
    let motion_cfg = config.clone();
    let motion_state = hover_state.clone();
    let active_state = active_submenu.clone();
    let widget_weak = drawing_area.downgrade();

    motion.connect_motion(move |controller, x, y| {
        let ui = &motion_cfg.ui;
        if ui.hover_mode == "none" { return; }

        let widget = match controller.widget() {
            Some(w) => w,
            None => return,
        };
        let w = widget.width() as f64;
        let h = widget.height() as f64;
        let center_x = w / 2.0;
        let center_y = h / 2.0;
        
        let dx = x - center_x;
        let dy = y - center_y;
        let dist = (dx * dx + dy * dy).sqrt();

        let mut new_target = HoverTarget::None;
        let current_active = *active_state.borrow();

        // Check Outer Ring First
        let sub_inner = ui.outer_radius + 5.0;
        let sub_outer = ui.outer_radius + 80.0;
        
        if current_active.is_some() && dist >= sub_inner && dist <= sub_outer {
             if let Some(parent_idx) = current_active {
                if let Some(parent) = motion_cfg.items.get(parent_idx) {
                    let sub_count = parent.items.len();
                    if sub_count > 0 {
                        let angle = dy.atan2(dx);
                        let mut active_angle = angle + PI / 2.0;
                        if active_angle < 0.0 { active_angle += 2.0 * PI; }
                        let angle_per_item = 2.0 * PI / sub_count as f64;
                        let index = (active_angle / angle_per_item).floor() as usize;
                        if index < sub_count {
                            new_target = HoverTarget::OuterRingItem(index);
                        }
                    }
                }
             }
        }
        else if dist < ui.tray_inner_radius {
            new_target = HoverTarget::Center;
        } else if dist <= ui.outer_radius {
            let items_count = motion_cfg.items.len();
            if items_count > 0 {
                let angle = dy.atan2(dx);
                let mut active_angle = angle + PI / 2.0;
                if active_angle < 0.0 { active_angle += 2.0 * PI; }
                let angle_per_item = 2.0 * PI / items_count as f64;
                let index = (active_angle / angle_per_item).floor() as usize;
                if index < items_count {
                    new_target = HoverTarget::RingItem(index);
                }
            }
        }

        let mut current = motion_state.borrow_mut();
        if *current != new_target {
            *current = new_target;
            if let Some(da) = widget_weak.upgrade() {
                da.queue_draw();
            }
        }
    });

    let leave_state = hover_state.clone();
    let leave_weak = drawing_area.downgrade();
    motion.connect_leave(move |_| {
        let mut current = leave_state.borrow_mut();
        if *current != HoverTarget::None {
            *current = HoverTarget::None;
            if let Some(da) = leave_weak.upgrade() {
                da.queue_draw();
            }
        }
    });

    drawing_area.add_controller(motion);
}

fn get_volume() -> f64 {
    let output = Command::new("pamixer")
        .arg("--get-volume")
        .output();

    if let Ok(output) = output {
        let s = String::from_utf8_lossy(&output.stdout);
        let vol: f64 = s.trim().parse().unwrap_or(0.0);
        return vol / 100.0;
    }
    0.0
}
