#![allow(clippy::type_complexity)]

use async_channel::Sender;
use futures_util::StreamExt;
use quick_xml::de::from_str;
use serde::Deserialize;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use zbus::fdo;
use zbus::proxy;
use zbus::{interface, Connection, SignalContext};

// -----------------------------------------------------------------------------
// Data Structures
// -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TrayItem {
    pub name: String,
    pub icon_name: String,
    pub title: String,
    pub status: String,
    pub path: String,
    pub service: String,
    pub menu_path: String,
}

pub type TrayItems = Arc<Mutex<Vec<TrayItem>>>;

// XML Structs for Introspection
#[derive(Debug, Deserialize)]
struct Node {
    #[serde(rename = "node", default)]
    nodes: Vec<Node>,
    #[serde(rename = "interface", default)]
    interfaces: Vec<Interface>,
    #[serde(rename = "@name")]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Interface {
    #[serde(rename = "@name")]
    name: String,
}

// -----------------------------------------------------------------------------
// Watcher Implementation (Server Side)
// -----------------------------------------------------------------------------

struct WatcherImpl {
    items: Arc<Mutex<HashSet<String>>>,
    hosts: Arc<Mutex<HashSet<String>>>,
}

impl WatcherImpl {
    fn new() -> Self {
        Self {
            items: Arc::new(Mutex::new(HashSet::new())),
            hosts: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl WatcherImpl {
    async fn register_status_notifier_item(
        &mut self,
        service: String,
        #[zbus(header)] header: zbus::MessageHeader<'_>,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
    ) -> fdo::Result<()> {
        let sender = header
            .sender()
            .ok_or_else(|| fdo::Error::Failed("No sender".into()))?
            .to_string();

        let (bus_name, object_path) = if service.starts_with('/') {
            (sender, service)
        } else if let Some((b, p)) = service.split_once('/') {
            (b.to_string(), format!("/{}", p))
        } else {
            (service, "/StatusNotifierItem".to_string())
        };

        let safe_path = if object_path.starts_with('/') {
            object_path
        } else {
            format!("/{}", object_path)
        };
        let key = format!("{}{}", bus_name, safe_path);

        let is_new = {
            let mut items = self.items.lock().unwrap();
            items.insert(key.clone())
        };

        if is_new {
            Self::status_notifier_item_registered(&ctxt, &key)
                .await
                .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        }
        Ok(())
    }

    async fn register_status_notifier_host(
        &mut self,
        service: String,
        #[zbus(header)] header: zbus::MessageHeader<'_>,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
    ) -> fdo::Result<()> {
        let sender = header
            .sender()
            .ok_or_else(|| fdo::Error::Failed("No sender".into()))?
            .to_string();
        let host_name = if service.is_empty() { sender } else { service };

        {
            let mut hosts = self.hosts.lock().unwrap();
            hosts.insert(host_name);
        }
        Self::status_notifier_host_registered(&ctxt)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        Ok(())
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.lock().unwrap().iter().cloned().collect()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        !self.hosts.lock().unwrap().is_empty()
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        ctxt: &SignalContext<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        ctxt: &SignalContext<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(ctxt: &SignalContext<'_>) -> zbus::Result<()>;
}

// -----------------------------------------------------------------------------
// Client Proxies
// -----------------------------------------------------------------------------

#[proxy(
    default_service = "org.kde.StatusNotifierWatcher",
    interface = "org.kde.StatusNotifierWatcher",
    default_path = "/StatusNotifierWatcher"
)]
trait StatusNotifierWatcher {
    /// RegisterStatusNotifierHost method
    fn register_status_notifier_host(&self, service: &str) -> zbus::Result<()>;

    /// RegisterStatusNotifierItem method
    fn register_status_notifier_item(&self, service: &str) -> zbus::Result<()>;

    /// StatusNotifierHostRegistered signal
    #[zbus(signal)]
    fn status_notifier_host_registered(&self) -> zbus::Result<()>;

    /// StatusNotifierHostUnregistered signal
    #[zbus(signal)]
    fn status_notifier_host_unregistered(&self) -> zbus::Result<()>;

    /// StatusNotifierItemRegistered signal
    #[zbus(signal)]
    fn status_notifier_item_registered(&self, service: &str) -> zbus::Result<()>;

    /// StatusNotifierItemUnregistered signal
    #[zbus(signal)]
    fn status_notifier_item_unregistered(&self, service: &str) -> zbus::Result<()>;

    /// IsStatusNotifierHostRegistered property
    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> zbus::Result<bool>;

    /// ProtocolVersion property
    #[zbus(property)]
    fn protocol_version(&self) -> zbus::Result<i32>;

    /// RegisteredStatusNotifierItems property
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> zbus::Result<Vec<String>>;
}

#[proxy(interface = "org.kde.StatusNotifierItem", assume_defaults = true)]
trait StatusNotifierItem {
    /// Activate method
    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// ContextMenu method
    fn context_menu(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// Scroll method
    fn scroll(&self, delta: i32, orientation: &str) -> zbus::Result<()>;

    /// SecondaryActivate method
    fn secondary_activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// NewAttentionIcon signal
    #[zbus(signal)]
    fn new_attention_icon(&self) -> zbus::Result<()>;

    /// NewIcon signal
    #[zbus(signal)]
    fn new_icon(&self) -> zbus::Result<()>;

    /// NewOverlayIcon signal
    #[zbus(signal)]
    fn new_overlay_icon(&self) -> zbus::Result<()>;

    /// NewStatus signal
    #[zbus(signal)]
    fn new_status(&self, status: &str) -> zbus::Result<()>;

    /// NewTitle signal
    #[zbus(signal)]
    fn new_title(&self) -> zbus::Result<()>;

    /// NewToolTip signal
    #[zbus(signal)]
    fn new_tool_tip(&self) -> zbus::Result<()>;

    /// AttentionIconName property
    #[zbus(property)]
    fn attention_icon_name(&self) -> zbus::Result<String>;

    /// AttentionIconPixmap property
    #[zbus(property)]
    fn attention_icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;

    /// AttentionMovieName property
    #[zbus(property)]
    fn attention_movie_name(&self) -> zbus::Result<String>;

    /// Category property
    #[zbus(property)]
    fn category(&self) -> zbus::Result<String>;

    /// IconName property
    #[zbus(property(emits_changed_signal = "false"))]
    fn icon_name(&self) -> zbus::Result<String>;

    /// IconPixmap property
    #[zbus(property(emits_changed_signal = "false"))]
    fn icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;

    /// IconThemePath property
    #[zbus(property)]
    fn icon_theme_path(&self) -> zbus::Result<String>;

    /// Id property
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    /// ItemIsMenu property
    #[zbus(property)]
    fn item_is_menu(&self) -> zbus::Result<bool>;

    /// Menu property
    #[zbus(property)]
    fn menu(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// OverlayIconName property
    #[zbus(property)]
    fn overlay_icon_name(&self) -> zbus::Result<String>;

    /// OverlayIconPixmap property
    #[zbus(property)]
    fn overlay_icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;

    /// Status property
    #[zbus(property)]
    fn status(&self) -> zbus::Result<String>;

    /// Title property
    #[zbus(property)]
    fn title(&self) -> zbus::Result<String>;

    /// ToolTip property
    #[zbus(property)]
    fn tool_tip(&self) -> zbus::Result<(String, Vec<(i32, i32, Vec<u8>)>)>;
}

// -----------------------------------------------------------------------------
// Main Logic
// -----------------------------------------------------------------------------

pub struct SNIWatcher {
    items: TrayItems,
    update_tx: Option<Sender<()>>,
}

impl SNIWatcher {
    pub fn new(update_tx: Option<Sender<()>>) -> Self {
        Self {
            items: Arc::new(Mutex::new(Vec::new())),
            update_tx,
        }
    }

    pub fn items(&self) -> TrayItems {
        self.items.clone()
    }

    pub async fn start(self) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 1. Establish Connection
        let conn = zbus::Connection::session().await?;

        // 2. Setup Watcher Implementation (Server)
        let watcher_impl = WatcherImpl::new();
        let _ = conn
            .object_server()
            .at("/StatusNotifierWatcher", watcher_impl)
            .await;

        // 3. Attempt to Acquire Name
        let reply = conn
            .request_name_with_flags(
                "org.kde.StatusNotifierWatcher",
                zbus::fdo::RequestNameFlags::DoNotQueue.into(),
            )
            .await?;

        match reply {
            zbus::fdo::RequestNameReply::PrimaryOwner => {
                println!("Waypie: Acquired org.kde.StatusNotifierWatcher (Running as Server)");
            }
            zbus::fdo::RequestNameReply::Exists => {
                println!("Waypie: Watcher already exists (Running as Client)");
            }
            zbus::fdo::RequestNameReply::AlreadyOwner => {
                println!("Waypie: Already owner (Running as Server)");
            }
            _ => {
                println!("Waypie: Unexpected name reply: {:?}", reply);
            }
        };

        // 4. Create Proxy to Watcher (Us or Them)
        let watcher_proxy = StatusNotifierWatcherProxy::new(&conn).await?;

        // 5. Register as Host
        if let Err(e) = watcher_proxy.register_status_notifier_host("waypie").await {
            eprintln!("Waypie: Failed to register host: {}", e);
        }

        // 6. Fetch Initial Items
        let initial_items = watcher_proxy.registered_status_notifier_items().await?;
        println!("Waypie: Initial items found: {}", initial_items.len());
        for item in initial_items {
            if let Err(e) = self.add_item(&conn, &item).await {
                eprintln!("Waypie: Error adding initial item {}: {}", item, e);
            }
        }
        if let Some(tx) = &self.update_tx {
            let _ = tx.send(()).await;
        }

        // 7. Subscribe to Signals (Event Loop)
        let mut registered_stream = watcher_proxy
            .receive_status_notifier_item_registered()
            .await?;
        let mut unregistered_stream = watcher_proxy
            .receive_status_notifier_item_unregistered()
            .await?;

        let items_store = self.items.clone();
        let conn_clone = conn.clone();
        let update_tx = self.update_tx.clone();

        // Spawn Signal Handler
        tokio::spawn(async move {
            loop {
                let mut changed = false;
                tokio::select! {
                    Some(msg) = registered_stream.next() => {
                        let service = msg.args().ok().map(|a| a.service.to_string()).unwrap_or_default();
                        println!("Waypie: Item Registered: {}", service);
                        if let Err(e) = Self::add_item_static(&conn_clone, &items_store, &service).await {
                            eprintln!("Waypie: Failed to add item {}: {}", service, e);
                        } else {
                            changed = true;
                        }
                    }
                    Some(msg) = unregistered_stream.next() => {
                        let service = msg.args().ok().map(|a| a.service.to_string()).unwrap_or_default();
                        println!("Waypie: Item Unregistered: {}", service);
                        Self::remove_item_static(&items_store, &service);
                        changed = true;
                    }
                }

                if changed {
                    if let Some(tx) = &update_tx {
                        let _ = tx.send(()).await;
                    }
                }
            }
        });

        // Keep alive
        std::future::pending::<()>().await;

        Ok(())
    }

    async fn add_item(
        &self,
        conn: &Connection,
        service_path: &str,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Self::add_item_static(conn, &self.items, service_path).await
    }

    // Static helper
    async fn add_item_static(
        conn: &Connection,
        items_store: &TrayItems,
        service_path: &str,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Initial parse of service/path
        let (service, mut path) = if let Some(idx) = service_path.find('/') {
            (
                service_path[..idx].to_string(),
                service_path[idx..].to_string(),
            )
        } else {
            (service_path.to_string(), "/StatusNotifierItem".to_string())
        };

        // Try standard path first
        let props_result = Self::fetch_props(conn, &service, &path).await;

        let (icon_name, title, status, menu_path) = match props_result {
            Ok(p) => p,
            Err(e) => {
                // If standard path failed, try to resolve via introspection
                println!("Waypie: Standard path failed for {}, resolving...", service);
                if let Some(resolved) = Self::resolve_pathless_address(conn, &service).await {
                    println!("Waypie: Resolved {} to {}", service, resolved);
                    path = resolved;
                    Self::fetch_props(conn, &service, &path).await?
                } else {
                    return Err(e); // Propagate original error if resolve fails
                }
            }
        };

        let tray_item = TrayItem {
            name: service_path.to_string(),
            icon_name,
            title,
            status,
            path,
            service,
            menu_path,
        };

        let mut list = items_store.lock().unwrap();
        // Deduplicate
        if !list.iter().any(|i| i.name == tray_item.name) {
            println!(
                "Waypie: Stored Tray Item: {} (Icon: {})",
                tray_item.name, tray_item.icon_name
            );
            list.push(tray_item);
        }

        Ok(())
    }

    async fn fetch_props(
        conn: &Connection,
        service: &str,
        path: &str,
    ) -> std::result::Result<
        (String, String, String, String),
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let item_proxy = StatusNotifierItemProxy::builder(conn)
            .destination(service)?
            .path(path)?
            .build()
            .await?;

        let icon_name = item_proxy.icon_name().await.unwrap_or_default();
        let title = item_proxy.title().await.unwrap_or_default();
        let status = item_proxy.status().await.unwrap_or_default();
        let menu_path = item_proxy
            .menu()
            .await
            .map(|p| p.to_string())
            .unwrap_or_else(|_| "".to_string());

        Ok((icon_name, title, status, menu_path))
    }

    // Auto-Discovery using Introspection
    async fn resolve_pathless_address(conn: &Connection, service: &str) -> Option<String> {
        let mut queue = VecDeque::new();
        queue.push_back("/".to_string());
        let mut checked = HashSet::new();

        // Limit depth/count to avoid huge scan
        let mut count = 0;

        while let Some(current_path) = queue.pop_front() {
            if !checked.insert(current_path.clone()) {
                continue;
            }
            if count > 100 {
                break;
            } // Safety break
            count += 1;

            let introspectable = zbus::fdo::IntrospectableProxy::builder(conn)
                .destination(service)
                .ok()?
                .path(current_path.clone())
                .ok()?
                .build()
                .await
                .ok()?;

            if let Ok(xml_data) = introspectable.introspect().await {
                if let Ok(node) = from_str::<Node>(&xml_data) {
                    // Check for interface
                    if node
                        .interfaces
                        .iter()
                        .any(|i| i.name == "org.kde.StatusNotifierItem")
                    {
                        return Some(current_path);
                    }

                    // Enqueue children
                    for child in node.nodes {
                        if let Some(name) = child.name {
                            let next_path = if current_path == "/" {
                                format!("/{}", name)
                            } else {
                                format!("{}/{}", current_path, name)
                            };
                            queue.push_back(next_path);
                        }
                    }
                }
            }
        }
        None
    }

    fn remove_item_static(items_store: &TrayItems, service_path: &str) {
        let mut list = items_store.lock().unwrap();
        if let Some(pos) = list.iter().position(|i| i.name == service_path) {
            println!("Waypie: Removed Tray Item: {}", service_path);
            list.remove(pos);
        }
    }
}

// -----------------------------------------------------------------------------
// Standalone Functions (Helpers for UI)
// -----------------------------------------------------------------------------

pub async fn activate_item(
    service: &str,
    path: &str,
    x: i32,
    y: i32,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let conn = Connection::session().await?;
    let item = StatusNotifierItemProxy::builder(&conn)
        .destination(service)?
        .path(path)?
        .build()
        .await?;

    match item.activate(x, y).await {
        Ok(_) => Ok(()),
        Err(e) => {
            println!(
                "Waypie: Primary activation failed for {}, trying secondary: {}",
                service, e
            );
            if let Err(e2) = item.secondary_activate(x, y).await {
                eprintln!(
                    "Waypie: Secondary activation also failed for {}: {}",
                    service, e2
                );
                return Err(Box::new(e2));
            }
            Ok(())
        }
    }
}

#[allow(dead_code)]
pub async fn context_menu(
    service: &str,
    path: &str,
    x: i32,
    y: i32,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let conn = Connection::session().await?;
    let item = StatusNotifierItemProxy::builder(&conn)
        .destination(service)?
        .path(path)?
        .build()
        .await?;

    if let Err(e) = item.context_menu(x, y).await {
        eprintln!("Waypie: ContextMenu failed for {}: {}", service, e);
    }
    Ok(())
}
