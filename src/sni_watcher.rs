use std::sync::{Arc, Mutex};
use zbus::proxy;
use zbus::Connection;

#[derive(Clone, Debug)]
pub struct TrayItem {
    #[allow(dead_code)]
    pub name: String,
    pub icon_name: String,
    pub title: String,
    #[allow(dead_code)]
    pub status: String,
    pub path: String,
    pub service: String,
}

pub type TrayItems = Arc<Mutex<Vec<TrayItem>>>;

/// SNI Watcher - discovers and tracks StatusNotifierItems from DBus
pub struct SNIWatcher {
    items: TrayItems,
    on_change: Option<Box<dyn Fn() + Send>>,
}

impl SNIWatcher {
    pub fn new() -> Self {
        Self {
            items: Arc::new(Mutex::new(Vec::new())),
            on_change: None,
        }
    }

    pub fn items(&self) -> TrayItems {
        self.items.clone()
    }

    /// Set a callback that fires whenever the tray items list changes
    #[allow(dead_code)]
    pub fn set_on_change<F: Fn() + Send + 'static>(&mut self, callback: F) {
        self.on_change = Some(Box::new(callback));
    }

    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        // Connect to session bus
        let conn = Connection::session().await?;

        // Register this application as a StatusNotifierHost
        self.register_host(&conn).await?;

        // Continuously poll for items
        self.poll_for_items(&conn).await?;

        Ok(())
    }

    async fn register_host(&self, conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
        let watcher = StatusNotifierWatcherProxy::new(conn).await?;
        watcher
            .register_status_notifier_host("/org/kde/StatusNotifierHost")
            .await
            .ok(); // Ignore if already registered
        Ok(())
    }

    async fn poll_for_items(&self, conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
        let mut last_items = Vec::new();

        loop {
            // Poll every 2 seconds for changes
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            match self.fetch_registered_items(conn).await {
                Ok(current_items) => {
                    // Check if items have changed
                    let mut changed = current_items.len() != last_items.len();

                    if !changed {
                        for (new, old) in current_items.iter().zip(last_items.iter()) {
                            if new != old {
                                changed = true;
                                break;
                            }
                        }
                    }

                    if changed {
                        // Find new items
                        for item_path in &current_items {
                            if !last_items.contains(item_path) {
                                if let Err(e) = self.add_item_from_path(conn, item_path).await {
                                    if std::env::var("WAYPIE_DEBUG").is_ok() {
                                        eprintln!("Failed to add SNI item {}: {}", item_path, e);
                                    }
                                }
                            }
                        }

                        // Find removed items
                        let mut items = self.items.lock().unwrap();
                        items.retain(|item| {
                            current_items.iter().any(|path| {
                                if let Some((service, obj_path)) = path.split_once('/') {
                                    format!("{}/{}", service, obj_path) == item.path
                                } else {
                                    path == &item.path
                                }
                            })
                        });
                        drop(items);

                        // Fire callback to trigger GTK redraw
                        if let Some(callback) = &self.on_change {
                            callback();
                        }

                        last_items = current_items;
                    }
                }
                Err(e) => {
                    if std::env::var("WAYPIE_DEBUG").is_ok() {
                        eprintln!("Failed to fetch registered items: {}", e);
                    }
                }
            }
        }
    }

    async fn fetch_registered_items(
        &self,
        conn: &Connection,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let watcher = StatusNotifierWatcherProxy::new(conn).await?;
        let items = watcher.registered_status_notifier_items().await?;
        Ok(items)
    }

    async fn add_item_from_path(
        &self,
        conn: &Connection,
        service_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Parse service path: could be "org.kde.StatusNotifierItem-1234-1"
        // or "org.kde.StatusNotifierItem-1234-1/org/kde/StatusNotifierItem"
        let (service, path) = if let Some((s, p)) = service_path.split_once('/') {
            (s, p)
        } else {
            (service_path, "/org/kde/StatusNotifierItem")
        };

        let (icon_name, title, status) = self.fetch_item_properties(conn, service, path).await?;

        let tray_item = TrayItem {
            name: service.to_string(),
            icon_name,
            title,
            status,
            path: path.to_string(),
            service: service.to_string(),
        };

        let mut items = self.items.lock().unwrap();
        if !items.iter().any(|i| i.path == tray_item.path) {
            items.push(tray_item);
        }

        Ok(())
    }

    async fn fetch_item_properties(
        &self,
        conn: &Connection,
        service: &str,
        path: &str,
    ) -> Result<(String, String, String), Box<dyn std::error::Error>> {
        let item = StatusNotifierItemProxy::builder(conn)
            .destination(service)?
            .path(path)?
            .build()
            .await?;

        let icon_name = item.icon_name().await.unwrap_or_default();
        let title = item.title().await.unwrap_or_default();
        let status = item.status().await.unwrap_or_default();

        Ok((icon_name, title, status))
    }

    #[allow(dead_code)]
    pub async fn activate_item(
        &self,
        service_path: &str,
        x: i32,
        y: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::session().await?;

        // Parse service path
        let (service, path) = if let Some((s, p)) = service_path.split_once('/') {
            (s, p)
        } else {
            (service_path, "/org/kde/StatusNotifierItem")
        };

        let item = StatusNotifierItemProxy::builder(&conn)
            .destination(service)?
            .path(path)?
            .build()
            .await?;

        item.activate(x, y).await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn context_menu(
        &self,
        service_path: &str,
        x: i32,
        y: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::session().await?;

        // Parse service path
        let (service, path) = if let Some((s, p)) = service_path.split_once('/') {
            (s, p)
        } else {
            (service_path, "/org/kde/StatusNotifierItem")
        };

        let item = StatusNotifierItemProxy::builder(&conn)
            .destination(service)?
            .path(path)?
            .build()
            .await?;

        item.context_menu(x, y).await?;

        Ok(())
    }
}

/// Standalone function to activate a StatusNotifierItem
pub async fn activate_item(service: &str, path: &str, x: i32, y: i32) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::session().await?;

    let item = StatusNotifierItemProxy::builder(&conn)
        .destination(service)?
        .path(path)?
        .build()
        .await?;

    item.activate(x, y).await?;

    Ok(())
}

/// StatusNotifierWatcher DBus interface
/// Service: org.kde.StatusNotifierWatcher
/// Path: /org/kde/StatusNotifierWatcher
#[proxy(
    interface = "org.kde.StatusNotifierWatcher",
    default_service = "org.kde.StatusNotifierWatcher",
    default_path = "/org/kde/StatusNotifierWatcher"
)]
trait StatusNotifierWatcher {
    /// Property: list of currently registered StatusNotifierItem services
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> zbus::Result<Vec<String>>;

    /// Method: register this application as a StatusNotifierHost
    fn register_status_notifier_host(&self, service: &str) -> zbus::Result<()>;

    /// Method: unregister this application as a StatusNotifierHost
    fn unregister_status_notifier_host(&self) -> zbus::Result<()>;
}

/// StatusNotifierItem DBus interface
/// Interface: org.kde.StatusNotifierItem
#[proxy(interface = "org.kde.StatusNotifierItem")]
trait StatusNotifierItem {
    /// Property: icon name following freedesktop.org icon naming spec
    #[zbus(property)]
    fn icon_name(&self) -> zbus::Result<String>;

    /// Property: title/name of the item
    #[zbus(property)]
    fn title(&self) -> zbus::Result<String>;

    /// Property: status of the item (Active, Passive, NeedsAttention)
    #[zbus(property)]
    fn status(&self) -> zbus::Result<String>;

    /// Method: activate the item at given coordinates
    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// Method: show context menu at given coordinates
    fn context_menu(&self, x: i32, y: i32) -> zbus::Result<()>;
}
