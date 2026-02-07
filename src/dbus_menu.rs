use gtk4::gdk::Rectangle;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Box, Button, Orientation, PolicyType, Popover, ScrolledWindow, Separator};
use serde::Deserialize;
use std::collections::HashMap;
use zbus::zvariant::{Array, Dict, OwnedValue, Structure, Value};
use zbus::{proxy, Connection, Result};

#[proxy(
    interface = "com.canonical.dbusmenu",
    default_path = "/com/canonical/dbusmenu"
)]
trait DBusMenu {
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: &[&str],
    ) -> Result<(u32, LayoutNode)>;
    fn event(&self, id: i32, event_id: &str, data: &Value<'_>, timestamp: u32) -> Result<()>;
}

#[derive(Debug, Deserialize, zbus::zvariant::Type)]
pub struct LayoutNode(i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

pub fn popup(service: String, path: String, parent_widget: &gtk4::Widget, x: f64, y: f64) {
    if path.is_empty() {
        eprintln!("Waypie: DBusMenu path is empty for {}", service);
        return;
    }

    let parent = parent_widget.clone();
    let s1 = service.clone();
    let p1 = path.clone();

    glib::MainContext::default().spawn_local(async move {
        if let Some(rt) = crate::RUNTIME.get() {
            let layout_future = rt.spawn(async move { fetch_layout(s1, p1).await });

            match layout_future.await {
                Ok(Ok(layout)) => {
                    println!(
                        "Waypie: Layout fetched. Root items count: {}",
                        layout.2.len()
                    );
                    build_and_show_popover(&parent, layout, service, path, x, y);
                }
                Ok(Err(e)) => eprintln!("Waypie: DBusMenu fetch error: {}", e),
                Err(e) => eprintln!("Waypie: Tokio join error: {}", e),
            }
        }
    });
}

pub async fn fetch_layout(service: String, path: String) -> Result<LayoutNode> {
    let conn = Connection::session().await?;
    let proxy = DBusMenuProxy::builder(&conn)
        .destination(service)?
        .path(path)?
        .build()
        .await?;

    let props = vec!["label", "enabled", "visible", "type", "children-display"];
    let (_rev, layout) = proxy.get_layout(0, -1, &props).await?;
    Ok(layout)
}

fn build_and_show_popover(
    parent: &gtk4::Widget,
    layout: LayoutNode,
    service: String,
    path: String,
    x: f64,
    y: f64,
) {
    let popover = Popover::builder().has_arrow(true).autohide(true).build();

    popover.set_parent(parent);

    let rect = Rectangle::new(x as i32, y as i32, 1, 1);
    popover.set_pointing_to(Some(&rect));

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(50)
        .max_content_height(400)
        .propagate_natural_width(true)
        .build();

    let root_box = Box::new(Orientation::Vertical, 0);

    for child_val in layout.2 {
        if let Some(widget) = build_menu_item(&child_val, &service, &path, 0, &popover) {
            root_box.append(&widget);
        }
    }

    scrolled.set_child(Some(&root_box));
    popover.set_child(Some(&scrolled));
    popover.popup();
}

fn peel_value<'a>(v: &'a Value<'a>) -> &'a Value<'a> {
    match v {
        Value::Value(inner) => peel_value(inner),
        _ => v,
    }
}

fn build_menu_item(
    val: &Value,
    service: &str,
    path: &str,
    depth: i32,
    popover: &Popover,
) -> Option<gtk4::Widget> {
    let inner_val = peel_value(val);

    let structure = match inner_val.downcast_ref::<Structure>() {
        Ok(s) => s,
        Err(_) => {
            eprintln!(
                "Waypie: Item at depth {} is not a Structure. Type: {}",
                depth,
                inner_val.value_signature()
            );
            return None;
        }
    };

    let fields = structure.fields();
    if fields.len() != 3 {
        eprintln!(
            "Waypie: Structure at depth {} has wrong field count: {}",
            depth,
            fields.len()
        );
        return None;
    }

    // Field 0: ID
    let id_val = peel_value(&fields[0]);
    let id = i32::try_from(id_val).ok()?;

    // Field 1: Props
    let props_val = peel_value(&fields[1]);
    let props = props_val.downcast_ref::<Dict>().ok()?;

    // Field 2: Children
    let children_val = peel_value(&fields[2]);

    let mut label = String::new();
    let mut enabled = true;
    let mut visible = true;
    let mut is_separator = false;

    for (k, v) in props.iter() {
        let key = String::try_from(k).unwrap_or_default();
        let peeled = peel_value(v);

        match key.as_str() {
            "label" => {
                label = String::try_from(peeled)
                    .unwrap_or_default()
                    .replace("_", "")
            }
            "enabled" => enabled = bool::try_from(peeled).unwrap_or(true),
            "visible" => visible = bool::try_from(peeled).unwrap_or(true),
            "type" => {
                if let Ok(t) = String::try_from(peeled) {
                    if t == "separator" {
                        is_separator = true;
                    }
                }
            }
            _ => {}
        }
    }

    if !visible {
        return None;
    }

    if is_separator {
        return Some(Separator::new(Orientation::Horizontal).into());
    }

    let container = Box::new(Orientation::Vertical, 0);
    container.set_margin_start((depth * 10) as i32);

    let mut widget_added = false;

    // Add Button if label exists
    if !label.is_empty() {
        let button = Button::builder()
            .label(&label)
            .has_frame(false)
            .halign(gtk4::Align::Fill)
            .build();

        // Align label text to start
        if let Some(child) = button.child() {
            if let Some(label_widget) = child.downcast_ref::<gtk4::Label>() {
                label_widget.set_xalign(0.0);
            }
        }

        button.set_sensitive(enabled);

        let s = service.to_string();
        let p = path.to_string();
        let popover_clone = popover.clone();

        button.connect_clicked(move |_| {
            let s = s.clone();
            let p = p.clone();
            let popover = popover_clone.clone();

            if let Some(rt) = crate::RUNTIME.get() {
                rt.spawn(async move {
                    if let Ok(conn) = Connection::session().await {
                        if let Ok(proxy) = DBusMenuProxy::builder(&conn)
                            .destination(s)
                            .unwrap()
                            .path(p)
                            .unwrap()
                            .build()
                            .await
                        {
                            let _ = proxy.event(id, "clicked", &Value::from(""), 0).await;
                        }
                    }
                });
            }
            popover.popdown();
        });

        container.append(&button);
        widget_added = true;
    }

    // Process Children
    if let Ok(children_array) = children_val.downcast_ref::<Array>() {
        for child in children_array.iter() {
            let child: &Value = child;
            if let Some(child_widget) = build_menu_item(child, service, path, depth + 1, popover) {
                container.append(&child_widget);
                widget_added = true;
            }
        }
    }

    if widget_added {
        Some(container.into())
    } else {
        None
    }
}
