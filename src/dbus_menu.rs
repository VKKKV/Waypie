use dbusmenu_glib_sys as ffi;
use gtk4::glib;
use gtk4::glib::prelude::*;
use gtk4::glib::translate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use zbus::Connection;

use crate::hud::radial_menu::PieItem;

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
    pub fn id(&self) -> i32 {
        unsafe { ffi::dbusmenu_menuitem_get_id(self.as_ptr()) }
    }

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

    pub fn convert_dbus_to_pie(root: &Menuitem, service: &str, path: &str) -> Vec<PieItem> {
        let mut items = Vec::new();
        for child in root.children() {
            // Filter hidden items
            if child.property_exist("visible") && !child.property_get_bool("visible") {
                continue;
            }

            let type_str = child.property_get("type").unwrap_or_default().to_string();
            if type_str == "separator" {
                continue;
            }

            let label = child
                .property_get("label")
                .unwrap_or_default()
                .to_string()
                .replace("_", "");

            let icon = child
                .property_get("icon-name")
                .unwrap_or_default()
                .to_string();

            let icon = if icon.is_empty() {
                "view-more-symbolic".to_string()
            } else {
                icon
            };

            // Construct action string: "dbus_signal|service|path|ID"
            let action = format!("dbus_signal|{}|{}|{}", service, path, child.id());

            let children = Self::convert_dbus_to_pie(&child, service, path);

            items.push(PieItem {
                label,
                icon,
                action,
                children,
                item_type: Some("dbus_item".to_string()),
                tray_id: None,
            });
        }
        items
    }
}

/// Tries to activate the item via SNI `Activate` method.
/// Returns `true` if activation succeeded.
pub async fn activate_or_popup(
    service: String,
    item_path: String,
    _menu_path: String,
    _parent_widget: gtk4::Widget,
    x: f64,
    y: f64,
) -> bool {
    let service_clone = service.clone();

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

    activate_result.is_ok()
}

pub async fn fetch_dbus_menu_as_pie(service: String, path: String) -> Result<Vec<PieItem>, String> {
    // 1. Call AboutToShow to trigger dynamic update
    // We do this async on Tokio before creating the client on Main Thread
    let service_clone = service.clone();
    let path_clone = path.clone();

    // Spawn AboutToShow on Tokio
    let _ = crate::RUNTIME
        .get()
        .expect("Runtime not initialized")
        .spawn(async move {
            if let Ok(conn) = Connection::session().await {
                let _ = conn
                    .call_method(
                        Some(service_clone.as_str()),
                        path_clone.as_str(),
                        Some("com.canonical.dbusmenu"),
                        "AboutToShow",
                        &(0i32),
                    )
                    .await;
            }
        });

    let client = Client::new(&service, &path)
        .ok_or_else(|| format!("Failed to create DBusMenu Client for {}", service))?;

    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx_rc = Rc::new(RefCell::new(Some(tx)));
    let debounce_timer = Rc::new(RefCell::new(None::<glib::SourceId>));

    let tx_clone = tx_rc.clone();
    let debounce_clone = debounce_timer.clone();
    let s_clone = service.clone();
    let p_clone = path.clone();

    // Shared handler for signals
    let handle_update = move |client: &Client| {
        // Cancel existing timer
        if let Some(id) = debounce_clone.borrow_mut().take() {
            id.remove();
        }

        let tx_inner = tx_clone.clone();
        let s_inner = s_clone.clone();
        let p_inner = p_clone.clone();
        let client_weak = client.downgrade(); // Use weak ref if Client supports it, but Client is wrapper.
                                              // Client wrapper is cheap to clone but we need to keep it alive?
                                              // Actually, the closure captures 'client' reference.
                                              // We need to clone client to pass into timeout.
        let client_owned = client.clone();

        let id = glib::source::timeout_add_local(Duration::from_millis(75), move || {
            if let Some(root) = client_owned.root() {
                if let Some(tx) = tx_inner.borrow_mut().take() {
                    let _ = tx.send(Ok(Menuitem::convert_dbus_to_pie(&root, &s_inner, &p_inner)));
                }
            }
            glib::ControlFlow::Break
        });

        *debounce_clone.borrow_mut() = Some(id);
    };

    let handler = Rc::new(handle_update);

    let h1 = handler.clone();
    client.connect_layout_updated(move |c| h1(c));

    let h2 = handler.clone();
    client.connect_root_changed(move |c, _| h2(c));

    // Initial check (also debounced to allow AboutToShow to have effect)
    if client.root().is_some() {
        handler(&client);
    }

    match rx.await {
        Ok(res) => res,
        Err(_) => Err("DBusMenu client dropped or signals disconnected".to_string()),
    }
}
