use ksni::{self, menu::*, Tray, TrayMethods};
use crate::config::{AppConfig, MenuItemConfig};
use crate::utils::execute_command;
use std::time::Duration;

pub struct DynamicTray {
    config: AppConfig,
}

impl Tray for DynamicTray {
    fn id(&self) -> String {
        "waypie-tray".into()
    }

    fn icon_name(&self) -> String {
        self.config.icon.clone()
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut menu_items = build_menu_items(&self.config.items);

        menu_items.push(MenuItem::Separator);
        menu_items.push(StandardItem {
            label: "Exit".into(),
            activate: Box::new(|_| std::process::exit(0)),
            ..Default::default()
        }.into());

        menu_items
    }
}

fn build_menu_items(items: &[MenuItemConfig]) -> Vec<MenuItem<DynamicTray>> {
    let mut menu_items = Vec::new();

    for item in items {
        if !item.items.is_empty() {
            // Submenu
            let sub_items = build_menu_items(&item.items);
            let submenu = SubMenu {
                label: item.label.clone(),
                submenu: sub_items,
                ..Default::default()
            };
            menu_items.push(submenu.into());
        } else {
            // Standard Item
            let script_cmd = item.script.clone();
            let menu_item = StandardItem {
                label: item.label.clone(),
                activate: Box::new(move |_| {
                    if let Some(cmd) = &script_cmd {
                        execute_command(cmd);
                    }
                }),
                ..Default::default()
            };
            menu_items.push(menu_item.into());
        }
    }

    menu_items
}

pub fn run_daemon() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let config = crate::config::load();

        // Start SNI Watcher
        let watcher = crate::sni_watcher::SNIWatcher::new();
        tokio::spawn(async move {
            if let Err(e) = watcher.start().await {
                eprintln!("SNI Watcher error: {}", e);
            }
        });

        // Start Tray
        let tray = DynamicTray { config };
        let _handle = tray.spawn().await;

        // Keep the process alive
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    })
}