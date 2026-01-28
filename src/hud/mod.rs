use crate::config::AppConfig;
use crate::utils::execute_command;
use chrono::Local;
use gdk_pixbuf::Pixbuf;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, DrawingArea, EventControllerMotion, EventControllerScroll,
    EventControllerScrollFlags, GestureClick,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::f64::consts::PI;
use std::process::Command;
use std::rc::Rc;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Debug)]
enum HudState {
    Idle,       // Normal wheel with center hub + ring
    TrayActive, // Outer ring showing tray apps
    #[allow(dead_code)]
    ContextActive(usize), // Context ring for tray app at index
}

#[derive(Clone, Copy, PartialEq)]
enum HoverTarget {
    None,
    Center,
    RingItem(usize),
    OuterRingItem(usize),
    TrayButton,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum HitResult {
    None,
    Center,
    TrayButton,
    RingItem(usize),
    OuterRingItem(usize),
    ContextMenuItem(usize),
}

pub fn run(app_id: &str, config: AppConfig, sni_items: crate::sni_watcher::TrayItems) {
    let app = Application::builder().application_id(app_id).build();

    app.connect_activate(move |app| build_ui(app, config.clone(), sni_items.clone()));
    app.run_with_args(&Vec::<String>::new());
}

fn build_ui(app: &Application, config: AppConfig, sni_items: crate::sni_watcher::TrayItems) {
    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(config.ui.width) // No longer needed for positioning
        .default_height(config.ui.height)
        .build();

    // Wayland layer shell integration (required for fullscreen overlay positioning)
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
    // Anchor to all edges to create a fullscreen overlay
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Bottom, true);
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
    let wheel_pos = Rc::new(RefCell::new(Option::<(f64, f64)>::None));
    let hud_state = Rc::new(RefCell::new(HudState::Idle));

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
    let draw_pos = wheel_pos.clone();
    let draw_hud_state = hud_state.clone();
    let draw_sni_items = sni_items.clone();

    drawing_area.set_draw_func(move |_, context, width, height| {
        let w = width as f64;
        let h = height as f64;

        // Use captured position or fallback to center
        let (center_x, center_y) = if let Some((px, py)) = *draw_pos.borrow() {
            (px, py)
        } else {
            (w / 2.0, h / 2.0)
        };

        let now = Local::now();
        let volume = get_volume();
        let current_hover = *draw_hover.borrow();
        let current_active = *draw_active.borrow();
        let current_hud_state = *draw_hud_state.borrow();

        draw_radial_wheel(
            context,
            center_x,
            center_y,
            &draw_config,
            now,
            volume,
            current_hover,
            current_active,
            current_hud_state,
            draw_sni_items.clone(),
        );
    });

    // Periodic redraw timer for SNI updates (every 500ms)
    let da_timer = drawing_area.downgrade();
    glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
        if let Some(da) = da_timer.upgrade() {
            da.queue_draw();
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });

    // Interaction (Click)
    setup_click_handler(
        &drawing_area,
        config_rc.clone(),
        wheel_pos.clone(),
        hud_state.clone(),
        sni_items.clone(),
    );
    // Interaction (Scroll)
    setup_scroll_handler(&drawing_area, config_rc.clone());
    // Interaction (Hover)
    setup_hover_handler(
        &drawing_area,
        config_rc.clone(),
        hover_state,
        hud_state.clone(),
        wheel_pos.clone(),
    );

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
    hud_state: HudState,
    sni_items: crate::sni_watcher::TrayItems,
) {
    let ui = &config.ui;
    let colors = &ui.colors;

    // 1. Background - clear the entire drawing area first
    context.set_source_rgba(0.0, 0.0, 0.0, 0.1);
    context.paint().expect("Failed to paint clear background");

    // Draw the background wheel circle
    context.set_source_rgba(
        colors.background.0,
        colors.background.1,
        colors.background.2,
        colors.background.3,
    );
    context.arc(cx, cy, ui.outer_radius, 0.0, 2.0 * PI);
    context.fill().expect("Failed to fill bg");

    // 2. Tray Items (Main Ring)
    let items_count = config.items.len();
    if items_count > 0 {
        let angle_per_item = 2.0 * PI / items_count as f64;
        let start_offset = -PI / 2.0; // Start at top (12 o'clock)

        for (i, item) in config.items.iter().enumerate() {
            let start_angle = start_offset + (i as f64 * angle_per_item);
            let end_angle = start_angle + angle_per_item;

            // Segment Colors
            if i % 2 == 0 {
                context.set_source_rgba(
                    colors.tray_even.0,
                    colors.tray_even.1,
                    colors.tray_even.2,
                    colors.tray_even.3,
                );
            } else {
                context.set_source_rgba(
                    colors.tray_odd.0,
                    colors.tray_odd.1,
                    colors.tray_odd.2,
                    colors.tray_odd.3,
                );
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
                        context.set_source_rgba(
                            colors.hover_overlay.0,
                            colors.hover_overlay.1,
                            colors.hover_overlay.2,
                            colors.hover_overlay.3,
                        );
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
            context.select_font_face(
                &ui.font_family,
                gtk4::cairo::FontSlant::Normal,
                gtk4::cairo::FontWeight::Bold,
            );
            context.set_font_size(13.0);
            let extents = context.text_extents(&item.label).unwrap();
            context.move_to(tx - extents.width() / 2.0, ty + extents.height() / 4.0);
            context.show_text(&item.label).unwrap();

            // Separator
            context.set_source_rgb(0.0, 0.0, 0.0);
            context.set_line_width(2.0);
            context.move_to(
                cx + ui.tray_inner_radius * start_angle.cos(),
                cy + ui.tray_inner_radius * start_angle.sin(),
            );
            context.line_to(
                cx + ui.outer_radius * start_angle.cos(),
                cy + ui.outer_radius * start_angle.sin(),
            );
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
                        context.set_source_rgba(
                            colors.tray_even.0,
                            colors.tray_even.1,
                            colors.tray_even.2,
                            colors.tray_even.3,
                        );
                    } else {
                        context.set_source_rgba(
                            colors.tray_odd.0,
                            colors.tray_odd.1,
                            colors.tray_odd.2,
                            colors.tray_odd.3,
                        );
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
                                context.set_source_rgba(
                                    colors.hover_overlay.0,
                                    colors.hover_overlay.1,
                                    colors.hover_overlay.2,
                                    colors.hover_overlay.3,
                                );
                                context.new_path();
                                context.arc(cx, cy, sub_outer_radius, start_angle, end_angle);
                                context.arc_negative(
                                    cx,
                                    cy,
                                    sub_inner_radius,
                                    end_angle,
                                    start_angle,
                                );
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
                    context.select_font_face(
                        &ui.font_family,
                        gtk4::cairo::FontSlant::Normal,
                        gtk4::cairo::FontWeight::Bold,
                    );
                    context.set_font_size(12.0);
                    let extents = context.text_extents(&item.label).unwrap();
                    context.move_to(tx - extents.width() / 2.0, ty + extents.height() / 4.0);
                    context.show_text(&item.label).unwrap();

                    // Separator
                    context.set_source_rgb(0.0, 0.0, 0.0);
                    context.set_line_width(2.0);
                    context.move_to(
                        cx + sub_inner_radius * start_angle.cos(),
                        cy + sub_inner_radius * start_angle.sin(),
                    );
                    context.line_to(
                        cx + sub_outer_radius * start_angle.cos(),
                        cy + sub_outer_radius * start_angle.sin(),
                    );
                    context.stroke().unwrap();
                }
            }
        }
    }

    // Hover Effect for Center
    if ui.hover_mode == "highlight" && hover == HoverTarget::Center {
        context.set_source_rgba(
            colors.hover_overlay.0,
            colors.hover_overlay.1,
            colors.hover_overlay.2,
            colors.hover_overlay.3,
        );
        context.new_path();
        context.arc(cx, cy, ui.tray_inner_radius, 0.0, 2.0 * PI);
        context.fill().unwrap();
    }

    // 4. Volume Arc (Inner Ring)
    // Track
    context.set_source_rgba(
        colors.volume_track.0,
        colors.volume_track.1,
        colors.volume_track.2,
        colors.volume_track.3,
    );
    context.set_line_width(8.0);
    context.set_line_cap(gtk4::cairo::LineCap::Round);
    context.arc(cx, cy, ui.vol_radius, 0.0, 2.0 * PI);
    context.stroke().unwrap();

    // Active Level
    if volume > 0.8 {
        context.set_source_rgb(
            colors.volume_warning.0,
            colors.volume_warning.1,
            colors.volume_warning.2,
        );
    } else {
        context.set_source_rgb(
            colors.volume_arc.0,
            colors.volume_arc.1,
            colors.volume_arc.2,
        );
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

    // ─────────────────────────────────────────────────────────────
    // LAYER 2: TRAY BUTTON AT 6 O'CLOCK
    // ─────────────────────────────────────────────────────────────

    // Draw tray button as a special segment at 6 o'clock (270°)
    let items_count = config.items.len();
    if items_count > 0 {
        let segment_angle = 2.0 * PI / (items_count + 1) as f64; // +1 for tray button
        let half_seg = segment_angle / 2.0;
        let tray_angle = 3.0 * PI / 2.0; // 270° = 3π/2

        let start = tray_angle - half_seg;
        let end = tray_angle + half_seg;

        // Draw tray button segment
        context.set_line_width(2.0);
        context.set_line_cap(gtk4::cairo::LineCap::Round);

        // Highlight if hovering
        if hover == HoverTarget::TrayButton {
            context.set_source_rgba(
                colors.hover_overlay.0,
                colors.hover_overlay.1,
                colors.hover_overlay.2,
                colors.hover_overlay.3,
            );
        } else {
            context.set_source_rgba(
                colors.tray_even.0,
                colors.tray_even.1,
                colors.tray_even.2,
                colors.tray_even.3,
            );
        }

        // Arc for tray button
        context.arc(cx, cy, ui.outer_radius - 10.0, start, end);
        context.stroke().unwrap();

        // Tray icon label
        context.set_font_size(12.0);
        context.set_source_rgb(colors.text.0, colors.text.1, colors.text.2);
        let label = "⋮"; // Three dots icon
        let ext = context.text_extents(label).unwrap();

        let label_radius = (ui.outer_radius + ui.tray_inner_radius) / 2.0;
        let label_x = cx + label_radius * tray_angle.cos();
        let label_y = cy + label_radius * tray_angle.sin();
        context.move_to(label_x - ext.width() / 2.0, label_y + ext.height() / 2.0);
        context.show_text(label).unwrap();
    }

    // ─────────────────────────────────────────────────────────────
    // LAYER 3: CONDITIONAL OUTER RINGS
    // ─────────────────────────────────────────────────────────────

    match hud_state {
        HudState::Idle => {
            // No outer ring in Idle state
        }
        HudState::TrayActive => {
            // Draw tray apps in outer ring
            draw_tray_apps_ring(context, cx, cy, ui, colors, hover, sni_items);
        }
        HudState::ContextActive(app_idx) => {
            // Draw context menu for specific app
            if let Some(app) = config.tray_apps.get(app_idx) {
                draw_context_menu_ring(context, cx, cy, app, ui, colors, hover);
            }
        }
    }
}

/// Draw tray apps as outer ring segments
fn draw_tray_apps_ring(
    context: &gtk4::cairo::Context,
    cx: f64,
    cy: f64,
    ui: &crate::config::UiConfig,
    colors: &crate::config::ColorConfig,
    hover: HoverTarget,
    sni_items: crate::sni_watcher::TrayItems,
) {
    let items = sni_items.lock().unwrap();
    let app_count = items.len();
    if app_count == 0 {
        return;
    }

    let segment_angle = 2.0 * PI / app_count as f64;
    let outer_start = ui.outer_radius + 20.0; // More spacing from main ring
    let outer_end = ui.outer_radius + 100.0; // Further outer ring
    let icon_radius = (outer_start + outer_end) / 2.0; // Icons in the middle

    context.set_line_width(2.0);
    context.set_line_cap(gtk4::cairo::LineCap::Round);

    for (i, item) in items.iter().enumerate() {
        let angle = (i as f64) * segment_angle;

        // Draw segment background
        let is_hovered = hover == HoverTarget::OuterRingItem(i);
        if is_hovered {
            context.set_source_rgba(
                colors.hover_overlay.0,
                colors.hover_overlay.1,
                colors.hover_overlay.2,
                colors.hover_overlay.3,
            );
        } else if i % 2 == 0 {
            context.set_source_rgba(
                colors.tray_even.0,
                colors.tray_even.1,
                colors.tray_even.2,
                colors.tray_even.3,
            );
        } else {
            context.set_source_rgba(
                colors.tray_odd.0,
                colors.tray_odd.1,
                colors.tray_odd.2,
                colors.tray_odd.3,
            );
        }

        let start = angle;
        let end = angle + segment_angle;
        context.arc(cx, cy, outer_start + 10.0, start, end);
        context.stroke().unwrap();

        // Calculate icon position
        let label_angle = angle + segment_angle / 2.0;
        let icon_x = cx + icon_radius * label_angle.cos();
        let icon_y = cy + icon_radius * label_angle.sin();

        // Try to render system icon, fallback to label text
        if !render_icon(context, &item.icon_name, icon_x, icon_y, 32) {
            // Fallback: render text label with title
            context.set_font_size(11.0);
            context.set_source_rgb(colors.text.0, colors.text.1, colors.text.2);

            let ext = context.text_extents(&item.title).unwrap();
            context.move_to(icon_x - ext.width() / 2.0, icon_y + ext.height() / 2.0);
            context.show_text(&item.title).unwrap();
        }
    }
}

/// Draw context menu actions as outer ring segments
fn draw_context_menu_ring(
    context: &gtk4::cairo::Context,
    cx: f64,
    cy: f64,
    app: &crate::config::TrayAppConfig,
    ui: &crate::config::UiConfig,
    colors: &crate::config::ColorConfig,
    hover: HoverTarget,
) {
    let action_count = app.actions.len();
    if action_count == 0 {
        return;
    }

    let segment_angle = 2.0 * PI / action_count as f64;
    let outer_start = ui.outer_radius + 20.0; // More spacing from main ring
    let outer_end = ui.outer_radius + 100.0; // Further outer ring
    let text_radius = (outer_start + outer_end) / 2.0; // Labels in the middle

    context.set_line_width(2.0);
    context.set_line_cap(gtk4::cairo::LineCap::Round);

    for (i, action) in app.actions.iter().enumerate() {
        let angle = (i as f64) * segment_angle;

        // Draw segment background
        let is_hovered = hover == HoverTarget::OuterRingItem(i);
        if is_hovered {
            context.set_source_rgba(
                colors.hover_overlay.0,
                colors.hover_overlay.1,
                colors.hover_overlay.2,
                colors.hover_overlay.3,
            );
        } else if i % 2 == 0 {
            context.set_source_rgba(
                colors.tray_even.0,
                colors.tray_even.1,
                colors.tray_even.2,
                colors.tray_even.3,
            );
        } else {
            context.set_source_rgba(
                colors.tray_odd.0,
                colors.tray_odd.1,
                colors.tray_odd.2,
                colors.tray_odd.3,
            );
        }

        let start = angle;
        let end = angle + segment_angle;
        context.arc(cx, cy, outer_start + 10.0, start, end);
        context.stroke().unwrap();

        // Draw label at segment position
        context.set_font_size(11.0);
        context.set_source_rgb(colors.text.0, colors.text.1, colors.text.2);

        let label_angle = angle + segment_angle / 2.0;
        let label_x = cx + text_radius * label_angle.cos();
        let label_y = cy + text_radius * label_angle.sin();

        let ext = context.text_extents(&action.label).unwrap();
        context.move_to(label_x - ext.width() / 2.0, label_y + ext.height() / 2.0);
        context.show_text(&action.label).unwrap();
    }
}

/// Load and render an icon from the system theme
/// Falls back to text label if icon not found
fn render_icon(context: &gtk4::cairo::Context, icon_name: &str, x: f64, y: f64, size: i32) -> bool {
    if icon_name.is_empty() {
        return false;
    }

    // Try to load pixbuf from standard icon paths
    if let Some(pixbuf) = load_pixbuf_from_paths(icon_name, size) {
        // Save context state
        context.save().unwrap();

        let px_width = pixbuf.width() as f64;
        let px_height = pixbuf.height() as f64;

        // Center the icon accounting for its actual dimensions
        context.translate(x - (px_width / 2.0), y - (px_height / 2.0));

        // Render the pixbuf directly
        if render_pixbuf_on_cairo(context, &pixbuf).is_ok() {
            context.restore().unwrap();
            return true;
        }

        context.restore().unwrap();
    }

    false
}

/// Render a pixbuf directly using Cairo
fn render_pixbuf_on_cairo(
    context: &gtk4::cairo::Context,
    pixbuf: &Pixbuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let width = pixbuf.width();
    let height = pixbuf.height();
    let rowstride = pixbuf.rowstride() as usize;

    // Get pixel data from pixbuf
    if let Some(pixel_bytes) = pixbuf.pixel_bytes() {
        let pixels = pixel_bytes.as_ref();
        let format = if pixbuf.has_alpha() {
            gtk4::cairo::Format::ARgb32
        } else {
            gtk4::cairo::Format::Rgb24
        };

        // Create surface from pixbuf data
        let surface = gtk4::cairo::ImageSurface::create_for_data(
            pixels.to_vec(),
            format,
            width,
            height,
            rowstride as i32,
        )?;

        context.set_source_surface(&surface, 0.0, 0.0)?;
        context.paint()?;
        Ok(())
    } else {
        Err("Failed to get pixel bytes".into())
    }
}

/// Try to load a pixbuf from standard icon paths
fn load_pixbuf_from_paths(icon_name: &str, size: i32) -> Option<Pixbuf> {
    let home = std::env::var("HOME").ok()?;

    // Try different standard icon locations
    let paths = vec![
        format!("{home}/.icons/hicolor/{size}/apps/{icon_name}.png"),
        format!("{home}/.icons/{icon_name}/apps/icon.png"),
        format!("/usr/share/icons/hicolor/{size}/apps/{icon_name}.png"),
        format!("/usr/share/icons/hicolor/48/apps/{icon_name}.png"),
        format!("/usr/share/icons/hicolor/32/apps/{icon_name}.png"),
        format!("/usr/share/icons/hicolor/24/apps/{icon_name}.png"),
        format!("/usr/share/pixmaps/{icon_name}.png"),
        format!("/usr/share/pixmaps/{icon_name}.svg"),
    ];

    for path in paths {
        if let Ok(pixbuf) = Pixbuf::from_file(&path) {
            // Scale preserving aspect ratio if needed
            let w = pixbuf.width();
            let h = pixbuf.height();
            if w != size || h != size {
                let scale = (size as f64 / w.max(h) as f64).min(1.0);
                let new_w = (w as f64 * scale) as i32;
                let new_h = (h as f64 * scale) as i32;
                if let Some(scaled) =
                    pixbuf.scale_simple(new_w, new_h, gdk_pixbuf::InterpType::Bilinear)
                {
                    return Some(scaled);
                }
            }
            return Some(pixbuf);
        }
    }

    None
}

fn setup_click_handler(
    drawing_area: &DrawingArea,
    config: Rc<AppConfig>,
    wheel_pos: Rc<RefCell<Option<(f64, f64)>>>,
    hud_state: Rc<RefCell<HudState>>,
    sni_items: crate::sni_watcher::TrayItems,
) {
    let click = GestureClick::new();
    click.set_button(0);

    let click_cfg = config.clone();
    let pos_state = wheel_pos.clone();
    let state_rc = hud_state.clone();
    let widget_weak = drawing_area.downgrade();

    click.connect_pressed(move |gesture, _, x, y| {
        let ui = &click_cfg.ui;

        let widget = gesture.widget().unwrap();
        let w = widget.width() as f64;
        let h = widget.height() as f64;

        let (center_x, center_y) = if let Some((px, py)) = *pos_state.borrow() {
            (px, py)
        } else {
            (w / 2.0, h / 2.0)
        };

        // Use polar coordinate hit detection
        let (radius, theta) = cartesian_to_polar(x, y, center_x, center_y);
        let button = gesture.current_button();
        let mut state = state_rc.borrow_mut();

        if button != gtk4::gdk::BUTTON_PRIMARY {
            return; // Only handle left-click for state transitions
        }

        // Route click based on state
        match *state {
            HudState::Idle => {
                // Detect hit in idle state
                let hit = detect_hit_main_ring(radius, theta, ui, click_cfg.items.len());
                match hit {
                    HitResult::Center => {
                        // Left click on center
                        if let Some(cmd) = &click_cfg.actions.left_click {
                            drop(state); // Release borrow before executing command
                            execute_command(cmd);
                        }
                    }
                    HitResult::TrayButton => {
                        // Open tray menu
                        *state = HudState::TrayActive;
                        drop(state);
                        if let Some(da) = widget_weak.upgrade() {
                            da.queue_draw();
                        }
                    }
                    HitResult::RingItem(idx) => {
                        // Execute main ring item or toggle submenu
                        if let Some(item) = click_cfg.items.get(idx) {
                            if let Some(script) = &item.script {
                                drop(state);
                                execute_command(script);
                                if let Some(da) = widget_weak.upgrade() {
                                    da.queue_draw();
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            HudState::TrayActive => {
                // Detect hit in tray state
                let items = sni_items.lock().unwrap();
                let item_count = items.len();

                let hit = if radius < ui.tray_inner_radius {
                    // Click on center closes tray
                    HitResult::Center
                } else {
                    detect_hit_tray_ring(radius, theta, ui, item_count)
                };

                match hit {
                    HitResult::Center => {
                        // Close tray, return to idle
                        *state = HudState::Idle;
                        drop(state);
                        drop(items);
                        if let Some(da) = widget_weak.upgrade() {
                            da.queue_draw();
                        }
                    }
                    HitResult::OuterRingItem(idx) => {
                        // Activate SNI item or open context menu
                        if idx < item_count {
                            if let Some(item) = items.get(idx) {
                                let service = item.service.clone();
                                let path = item.path.clone();
                                drop(state);
                                drop(items);

                                // Spawn async activation
                                tokio::spawn(async move {
                                    if let Err(e) = crate::sni_watcher::activate_item(
                                        &service,
                                        &path,
                                        center_x as i32,
                                        center_y as i32,
                                    )
                                    .await
                                    {
                                        if std::env::var("WAYPIE_DEBUG").is_ok() {
                                            eprintln!("Failed to activate SNI item: {}", e);
                                        }
                                    }
                                });

                                if let Some(da) = widget_weak.upgrade() {
                                    da.queue_draw();
                                }
                            } else {
                                drop(items);
                            }
                        } else {
                            drop(items);
                        }
                    }
                    _ => {
                        drop(items);
                    }
                }
            }
            HudState::ContextActive(app_idx) => {
                // Detect hit in context menu state
                if radius < ui.tray_inner_radius {
                    // Click on center closes context menu, return to tray
                    *state = HudState::TrayActive;
                    drop(state);
                    if let Some(da) = widget_weak.upgrade() {
                        da.queue_draw();
                    }
                } else if let Some(app) = click_cfg.tray_apps.get(app_idx) {
                    let hit = detect_hit_context_ring(radius, theta, ui, app.actions.len());
                    match hit {
                        HitResult::OuterRingItem(action_idx) => {
                            // Execute context action
                            if let Some(action) = app.actions.get(action_idx) {
                                drop(state);
                                execute_command(&action.command);
                                if let Some(da) = widget_weak.upgrade() {
                                    da.queue_draw();
                                }
                            }
                        }
                        _ => {}
                    }
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
    hud_state: Rc<RefCell<HudState>>,
    wheel_pos: Rc<RefCell<Option<(f64, f64)>>>,
) {
    let motion = EventControllerMotion::new();
    let motion_cfg = config.clone();
    let motion_state = hover_state.clone();
    let hud_rc = hud_state.clone();
    let pos_state = wheel_pos.clone();
    let widget_weak = drawing_area.downgrade();

    motion.connect_motion(move |controller, x, y| {
        let ui = &motion_cfg.ui;
        if ui.hover_mode == "none" {
            return;
        }

        let widget = match controller.widget() {
            Some(w) => w,
            None => return,
        };
        let w = widget.width() as f64;
        let h = widget.height() as f64;

        let (center_x, center_y) = if let Some((px, py)) = *pos_state.borrow() {
            (px, py)
        } else {
            (w / 2.0, h / 2.0)
        };

        // Use polar coordinates
        let (radius, theta) = cartesian_to_polar(x, y, center_x, center_y);

        let mut new_target = HoverTarget::None;
        let current_state = hud_rc.borrow();

        // Detect hover target based on current state
        match *current_state {
            HudState::Idle => {
                // Check main ring
                if let HitResult::TrayButton =
                    detect_hit_main_ring(radius, theta, ui, motion_cfg.items.len())
                {
                    new_target = HoverTarget::TrayButton;
                } else if let HitResult::RingItem(idx) =
                    detect_hit_main_ring(radius, theta, ui, motion_cfg.items.len())
                {
                    new_target = HoverTarget::RingItem(idx);
                } else if radius < ui.tray_inner_radius {
                    new_target = HoverTarget::Center;
                }
            }
            HudState::TrayActive => {
                // Check tray ring or center
                if radius < ui.tray_inner_radius {
                    new_target = HoverTarget::Center;
                } else if let HitResult::OuterRingItem(idx) =
                    detect_hit_tray_ring(radius, theta, ui, motion_cfg.tray_apps.len())
                {
                    new_target = HoverTarget::OuterRingItem(idx);
                }
            }
            HudState::ContextActive(app_idx) => {
                // Check context ring or center
                if radius < ui.tray_inner_radius {
                    new_target = HoverTarget::Center;
                } else if let Some(app) = motion_cfg.tray_apps.get(app_idx) {
                    if let HitResult::OuterRingItem(idx) =
                        detect_hit_context_ring(radius, theta, ui, app.actions.len())
                    {
                        new_target = HoverTarget::OuterRingItem(idx);
                    }
                }
            }
        }

        drop(current_state);

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
    let output = Command::new("pamixer").arg("--get-volume").output();

    if let Ok(output) = output {
        let s = String::from_utf8_lossy(&output.stdout);
        let vol: f64 = s.trim().parse().unwrap_or(0.0);
        return vol / 100.0;
    }
    0.0
}

// ─────────────────────────────────────────────────────────────
// COORDINATE SYSTEM & HIT DETECTION
// ─────────────────────────────────────────────────────────────

/// Convert Cartesian (x, y) to Polar (r, θ)
/// Returns (radius, theta_degrees) where θ is in degrees [0, 360)
/// θ = 0° is at 3 o'clock, 90° is at 12 o'clock, 270° is at 6 o'clock
fn cartesian_to_polar(x: f64, y: f64, center_x: f64, center_y: f64) -> (f64, f64) {
    let dx = x - center_x;
    let dy = y - center_y;

    let radius = (dx * dx + dy * dy).sqrt();

    // atan2(dy, dx) returns radians in range [-π, π]
    // We convert to degrees in range [0, 360)
    let theta_rad = dy.atan2(dx);
    let mut theta_deg = theta_rad.to_degrees();

    // Normalize to [0, 360) range, handling wrapping at 0-degree line
    if theta_deg < 0.0 {
        theta_deg += 360.0;
    }

    (radius, theta_deg)
}

/// Detect what was hit based on polar coordinates
/// Returns HitResult indicating the target of the interaction
#[allow(dead_code)]
fn detect_hit(
    x: f64,
    y: f64,
    center_x: f64,
    center_y: f64,
    config: &AppConfig,
    state: HudState,
) -> HitResult {
    let (radius, theta) = cartesian_to_polar(x, y, center_x, center_y);
    let ui = &config.ui;

    // Check center (always hittable for back/close)
    if radius < 40.0 {
        return HitResult::Center;
    }

    match state {
        HudState::Idle => {
            // In Idle state: check for main ring items + tray button
            detect_hit_main_ring(radius, theta, ui, config.items.len())
        }
        HudState::TrayActive => {
            // In TrayActive state: check for tray apps in outer ring
            detect_hit_tray_ring(radius, theta, ui, config.tray_apps.len())
        }
        HudState::ContextActive(app_idx) => {
            // In ContextActive state: check for context menu actions
            if let Some(app) = config.tray_apps.get(app_idx) {
                detect_hit_context_ring(radius, theta, ui, app.actions.len())
            } else {
                HitResult::None
            }
        }
    }
}

/// Hit detection for main ring (Idle state)
/// Detects: main ring items (3 o'clock to 6 o'clock) + tray button at 6 o'clock
fn detect_hit_main_ring(
    radius: f64,
    theta: f64,
    ui: &crate::config::UiConfig,
    item_count: usize,
) -> HitResult {
    // Tray button: segment centered at 270° (6 o'clock)
    // Each segment spans 360 / (item_count + 1) degrees
    if item_count == 0 {
        return HitResult::None;
    }

    let segment_angle = 360.0 / (item_count as f64 + 1.0); // +1 for tray button
    let half_segment = segment_angle / 2.0;

    // Check if in radial range
    if radius < ui.tray_inner_radius || radius > ui.outer_radius {
        return HitResult::None;
    }

    // Tray button at 270° (6 o'clock)
    let tray_start = 270.0 - half_segment;
    let tray_end = 270.0 + half_segment;

    if (theta >= tray_start && theta <= tray_end)
        || (tray_start < 0.0 && (theta >= tray_start + 360.0 || theta <= tray_end))
    {
        return HitResult::TrayButton;
    }

    // Check main ring items (skip tray button slot at 270°)
    for i in 0..item_count {
        let item_angle = (i as f64 * segment_angle) + segment_angle / 2.0;
        let item_start = item_angle - half_segment;
        let item_end = item_angle + half_segment;

        // Skip the tray button slot
        if (item_start..item_end).contains(&270.0) {
            continue;
        }

        if (theta >= item_start && theta <= item_end)
            || (item_start < 0.0 && (theta >= item_start + 360.0 || theta <= item_end))
            || (item_end > 360.0 && (theta >= item_start || theta <= item_end - 360.0))
        {
            return HitResult::RingItem(i);
        }
    }

    HitResult::None
}

/// Hit detection for tray ring (TrayActive state)
/// Detects: tray apps in outer ring
fn detect_hit_tray_ring(
    radius: f64,
    theta: f64,
    ui: &crate::config::UiConfig,
    app_count: usize,
) -> HitResult {
    if app_count == 0 {
        return HitResult::None;
    }

    // Outer ring with new spacing
    let outer_start = ui.outer_radius + 20.0;
    let outer_end = ui.outer_radius + 100.0;

    if radius < outer_start || radius > outer_end {
        return HitResult::None;
    }

    let segment_angle = 360.0 / app_count as f64;
    let half_segment = segment_angle / 2.0;

    for i in 0..app_count {
        let item_angle = (i as f64 * segment_angle) + segment_angle / 2.0;
        let item_start = item_angle - half_segment;
        let item_end = item_angle + half_segment;

        if (theta >= item_start && theta <= item_end)
            || (item_start < 0.0 && (theta >= item_start + 360.0 || theta <= item_end))
            || (item_end > 360.0 && (theta >= item_start || theta <= item_end - 360.0))
        {
            return HitResult::OuterRingItem(i);
        }
    }

    HitResult::None
}

/// Hit detection for context menu ring (ContextActive state)
/// Detects: context menu actions
fn detect_hit_context_ring(
    radius: f64,
    theta: f64,
    ui: &crate::config::UiConfig,
    action_count: usize,
) -> HitResult {
    if action_count == 0 {
        return HitResult::None;
    }

    // Context ring uses outer ring space with new spacing
    let outer_start = ui.outer_radius + 20.0;
    let outer_end = ui.outer_radius + 100.0;

    if radius < outer_start || radius > outer_end {
        return HitResult::None;
    }

    let segment_angle = 360.0 / action_count as f64;
    let half_segment = segment_angle / 2.0;

    for i in 0..action_count {
        let item_angle = (i as f64 * segment_angle) + segment_angle / 2.0;
        let item_start = item_angle - half_segment;
        let item_end = item_angle + half_segment;

        if (theta >= item_start && theta <= item_end)
            || (item_start < 0.0 && (theta >= item_start + 360.0 || theta <= item_end))
            || (item_end > 360.0 && (theta >= item_start || theta <= item_end - 360.0))
        {
            return HitResult::ContextMenuItem(i);
        }
    }

    HitResult::None
}
