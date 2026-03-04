use async_channel::Sender;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use system_tray::client::{Client, Event, UpdateEvent};
use system_tray::item::StatusNotifierItem;
use system_tray::menu::TrayMenu;

// -----------------------------------------------------------------------------
// Data Structures
// -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TrayItem {
    pub name: String,
    pub icon_name: String,
    pub title: String,
    pub path: String,
    pub service: String,
    pub menu_path: String,
}

pub type TrayItemsStore = Arc<Mutex<HashMap<String, (StatusNotifierItem, Option<TrayMenu>)>>>;

pub struct AppState {
    pub items: TrayItemsStore,
    pub client: Arc<Mutex<Option<Arc<Client>>>>,
}

pub struct SNIWatcher {
    pub state: Arc<AppState>,
    update_tx: Option<Sender<()>>,
}

impl SNIWatcher {
    pub fn new(update_tx: Option<Sender<()>>) -> Self {
        Self {
            state: Arc::new(AppState {
                items: Arc::new(Mutex::new(HashMap::new())),
                client: Arc::new(Mutex::new(None)),
            }),
            update_tx,
        }
    }

    pub fn get_legacy_items(&self) -> Vec<TrayItem> {
        let store = self.state.items.lock().unwrap();
        let mut items = Vec::with_capacity(store.len());

        for (key, (item, _)) in store.iter() {
            let (service, path) = key
                .split_once('/')
                .unwrap_or((key.as_str(), "/StatusNotifierItem"));

            let normalized_path = if path.starts_with('/') {
                path.to_string()
            } else {
                let mut p = String::with_capacity(path.len() + 1);
                p.push('/');
                p.push_str(path);
                p
            };

            items.push(TrayItem {
                name: key.clone(),
                icon_name: item.icon_name.clone().unwrap_or_default(),
                title: item.title.clone().unwrap_or_else(|| item.id.clone()),
                path: normalized_path,
                service: service.to_string(),
                menu_path: item
                    .menu
                    .as_ref()
                    .map(|p| p.to_string())
                    .unwrap_or_default(),
            });
        }

        items
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = Arc::new(Client::new().await?);
        let mut events = client.subscribe();

        {
            let mut stored_client = self.state.client.lock().unwrap();
            *stored_client = Some(client.clone());
        }

        println!("Waypie: system-tray client started.");

        let items_store = self.state.items.clone();
        let update_tx = self.update_tx.clone();

        while let Ok(event) = events.recv().await {
            let changed = match event {
                Event::Add(name, item) => {
                    let mut store = items_store.lock().unwrap();
                    store.insert(name, (*item, None)).is_none()
                }
                Event::Update(name, update) => {
                    let mut store = items_store.lock().unwrap();
                    if let Some((item, menu)) = store.get_mut(&name) {
                        match update {
                            UpdateEvent::Status(s) => {
                                if item.status != s {
                                    item.status = s;
                                    true
                                } else {
                                    false
                                }
                            }
                            UpdateEvent::Title(t) => {
                                if item.title != t {
                                    item.title = t;
                                    true
                                } else {
                                    false
                                }
                            }
                            UpdateEvent::Icon { icon_name, .. } => {
                                if item.icon_name != icon_name {
                                    item.icon_name = icon_name;
                                    true
                                } else {
                                    false
                                }
                            }
                            UpdateEvent::Menu(m) => {
                                *menu = Some(m);
                                true
                            }
                            _ => false,
                        }
                    } else {
                        false
                    }
                }
                Event::Remove(name) => {
                    let mut store = items_store.lock().unwrap();
                    store.remove(&name).is_some()
                }
            };

            if changed {
                if let Some(tx) = &update_tx {
                    let _ = tx.try_send(());
                }
            }
        }

        Ok(())
    }
}
