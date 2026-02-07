use dbusmenu_glib_sys as ffi;
use gtk4::gdk::Rectangle;
use gtk4::glib;
use gtk4::glib::translate::*;
use gtk4::prelude::*;
use gtk4::{Box, Button, Orientation, PolicyType, Popover, ScrolledWindow, Separator};
use zbus::Connection;

// Wrappers for dbusmenu-glib
glib::wrapper! {
    pub struct Client(Object<ffi::DbusmenuClient, ffi::DbusmenuClientClass>);
    match fn {
        type_ => || ffi::dbusmenu_client_get_type(),
    }
}

impl Client {
    pub fn new(name: &str, object: &str) -> Option<Client> {
        unsafe {
            let ptr = ffi::dbusmenu_client_new(name.to_glib_none().0, object.to_glib_none().0);
            if ptr.is_null() {
                None
            } else {
                Some(from_glib_full(ptr))
            }
        }
    }

    pub fn root(&self) -> Option<Menuitem> {
        unsafe { from_glib_none(ffi::dbusmenu_client_get_root(self.as_ptr())) }
    }

    pub fn connect_layout_updated<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("layout-updated", false, move |values| {
            let client = values[0]
                .get::<Client>()
                .expect("Failed to downcast to Client");
            f(&client);
            None
        })
    }

    pub fn connect_root_changed<F: Fn(&Self, &Menuitem) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("root-changed", false, move |values| {
            let client = values[0]
                .get::<Client>()
                .expect("Failed to downcast to Client");
            let root = values[1]
                .get::<Menuitem>()
                .expect("Failed to downcast to Menuitem");
            f(&client, &root);
            None
        })
    }
}

glib::wrapper! {
    pub struct Menuitem(Object<ffi::DbusmenuMenuitem, ffi::DbusmenuMenuitemClass>);
    match fn {
        type_ => || ffi::dbusmenu_menuitem_get_type(),
    }
}

impl Menuitem {
    pub fn children(&self) -> Vec<Menuitem> {
        unsafe {
            FromGlibPtrContainer::from_glib_none(ffi::dbusmenu_menuitem_get_children(self.as_ptr()))
        }
    }

    pub fn property_get(&self, property: &str) -> Option<glib::GString> {
        unsafe {
            from_glib_none(ffi::dbusmenu_menuitem_property_get(
                self.as_ptr(),
                property.to_glib_none().0,
            ))
        }
    }

    pub fn property_get_bool(&self, property: &str) -> bool {
        unsafe {
            from_glib(ffi::dbusmenu_menuitem_property_get_bool(
                self.as_ptr(),
                property.to_glib_none().0,
            ))
        }
    }

    pub fn property_exist(&self, property: &str) -> bool {
        unsafe {
            from_glib(ffi::dbusmenu_menuitem_property_exist(
                self.as_ptr() as *const _,
                property.to_glib_none().0,
            ))
        }
    }

    pub fn handle_event(&self, name: &str, variant: &glib::Variant, timestamp: u32) {
        unsafe {
            ffi::dbusmenu_menuitem_handle_event(
                self.as_ptr(),
                name.to_glib_none().0,
                variant.to_glib_none().0,
                timestamp,
            );
        }
    }
}

/// Tries to activate the item via SNI `Activate` method.
/// If that fails (e.g. method not found), falls back to showing the DBusMenu popup.
/// Returns `true` if activation succeeded, `false` if fallback was used.
pub async fn activate_or_popup(
    service: String,
    item_path: String,
    menu_path: String,
    parent_widget: gtk4::Widget,
    x: f64,
    y: f64,
) -> bool {
    let parent_weak = parent_widget.downgrade();
    drop(parent_widget);

    let service_clone = service.clone();
    let menu_path_clone = menu_path.clone();

    let service_for_task = service.clone();
    let item_path_for_task = item_path.clone();
    let x_int = x as i32;
    let y_int = y as i32;

    println!(
        "Waypie: Attempting Activate for {} at {}...",
        service_for_task, item_path_for_task
    );

    let (tx, rx) = tokio::sync::oneshot::channel();

    crate::RUNTIME
        .get()
        .expect("Runtime not initialized")
        .spawn(async move {
            let result = async {
                let conn = Connection::session().await.map_err(|e| e.to_string())?;
                conn.call_method(
                    Some(service_for_task.as_str()),
                    item_path_for_task.as_str(),
                    Some("org.kde.StatusNotifierItem"),
                    "Activate",
                    &(x_int, y_int),
                )
                .await
                .map_err(|e| e.to_string())
            }
            .await;

            let _ = tx.send(result);
        });

    let activate_result = match rx.await {
        Ok(res) => res,
        Err(_) => Err("Tokio task cancelled".to_string()),
    };

    println!(
        "Waypie: Activate Result for {}: {:?}",
        service_clone, activate_result
    );

    if let Err(_) = activate_result {
        if let Some(parent) = parent_weak.upgrade() {
            glib::source::idle_add_local(move || {
                show_menu(
                    service_clone.clone(),
                    menu_path_clone.clone(),
                    &parent,
                    x,
                    y,
                );
                glib::ControlFlow::Break
            });
        }
        return false;
    } else {
        println!(
            "Waypie: Activate OK for {}. HUD should close if app handled it.",
            service_clone
        );
    }

    true
}

pub fn show_menu(service: String, path: String, _parent_widget: &gtk4::Widget, _x: f64, _y: f64) {
    if path.is_empty() {
        return;
    }

    println!("Waypie: Opening Debug Window for {} at {}...", service, path);

    let client = match Client::new(&service, &path) {
        Some(c) => c,
        None => {
            eprintln!(
                "Waypie: Failed to create DBusMenu Client for {} at {}",
                service, path
            );
            return;
        }
    };

    // Create a standalone Window instead of a Popover
    let window = gtk4::Window::builder()
        .title("WaypieDebugMenu")
        .decorated(false)
        .resizable(false)
        .default_width(200)
        .default_height(300)
        .build();

    // DEBUG: Visual CSS
    let provider = gtk4::CssProvider::new();
    provider.load_from_data("window { border: 5px solid red; }");
    window
        .style_context()
        .add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);

    let client_clone = client.clone();
    // Keep client alive with window
    window.connect_close_request(move |_| {
        let _ = &client_clone;
        glib::Propagation::Proceed
    });

    let window_weak = window.downgrade();

    let update_ui = move |client: &Client| {
        if let Some(window) = window_weak.upgrade() {
            if let Some(root) = client.root() {
                println!("Waypie: Client Root found! Building content...");
                let content = build_menu_content(&window, &root);
                window.set_child(Some(&content));
            } else {
                println!("Waypie: Waiting for root update...");
            }
        }
    };

    let update_ui_clone = update_ui.clone();
    client.connect_layout_updated(move |client| {
        update_ui_clone(client);
    });

    let update_ui_clone = update_ui.clone();
    client.connect_root_changed(move |client, _new_root| {
        update_ui_clone(client);
    });

    if let Some(root) = client.root() {
        println!("Waypie: Initial Root found!");
        let content = build_menu_content(&window, &root);
        window.set_child(Some(&content));
    } else {
        println!("Waypie: No initial root, showing spinner.");
        let loading_box = Box::new(Orientation::Vertical, 10);
        loading_box.set_margin_top(10);
        loading_box.set_margin_bottom(10);
        loading_box.set_margin_start(10);
        loading_box.set_margin_end(10);
        
        let close_btn = Button::with_label("Close");
        let win_clone = window.clone();
        close_btn.connect_clicked(move |_| win_clone.close());
        loading_box.append(&close_btn);

        let spinner = gtk4::Spinner::new();
        spinner.start();
        loading_box.append(&spinner);
        let label = gtk4::Label::new(Some("Loading menu..."));
        loading_box.append(&label);
        window.set_child(Some(&loading_box));
    }

    window.present();
}

fn build_menu_content(window: &gtk4::Window, root: &Menuitem) -> gtk4::Widget {
    let children = root.children();
    println!("Waypie: Root has {} children", children.len());

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .propagate_natural_width(true)
        .build();

    let root_box = Box::new(Orientation::Vertical, 0);

    let close_btn = Button::with_label("Close Menu");
    let win_clone = window.clone();
    close_btn.connect_clicked(move |_| win_clone.close());
    root_box.append(&close_btn);
    root_box.append(&Separator::new(Orientation::Horizontal));

    for child in children {
        if let Some(widget) = build_menu_item(&child, 0, window) {
            root_box.append(&widget);
        }
    }

    scrolled.set_child(Some(&root_box));
    scrolled.into()
}

fn build_menu_item(item: &Menuitem, depth: i32, window: &gtk4::Window) -> Option<gtk4::Widget> {
    let label = item
        .property_get("label")
        .unwrap_or_default()
        .to_string()
        .replace("_", "");

    let visible = if item.property_exist("visible") {
        item.property_get_bool("visible")
    } else {
        true
    };

    let type_str = item.property_get("type").unwrap_or_default();

    println!(
        "Waypie: Building Item | Label: '{}' | Type: '{}' | Visible: {}",
        label, type_str, visible
    );

    let enabled = if item.property_exist("enabled") {
        item.property_get_bool("enabled")
    } else {
        true
    };

    let is_separator = type_str == "separator";

    if !visible {
        return None;
    }

    if is_separator {
        return Some(Separator::new(Orientation::Horizontal).into());
    }

    let container = Box::new(Orientation::Vertical, 0);
    container.set_margin_start(depth * 10);

    let mut widget_added = false;

    if !label.is_empty() {
        let button = Button::builder()
            .label(&label)
            .has_frame(false)
            .halign(gtk4::Align::Fill)
            .build();

        if let Some(child) = button.child() {
            if let Some(label_widget) = child.downcast_ref::<gtk4::Label>() {
                label_widget.set_xalign(0.0);
            }
        }

        button.set_sensitive(enabled);

        let item_clone = item.clone();
        let window_weak = window.downgrade();

        button.connect_clicked(move |_| {
            item_clone.handle_event("clicked", &glib::Variant::from(""), 0);
            if let Some(win) = window_weak.upgrade() {
                win.close();
            }
        });

        container.append(&button);
        widget_added = true;
    }

    let children = item.children();
    for child in children {
        if let Some(child_widget) = build_menu_item(&child, depth + 1, window) {
            container.append(&child_widget);
            widget_added = true;
        }
    }

    if widget_added {
        Some(container.into())
    } else {
        None
    }
}

