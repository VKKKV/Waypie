/// Menu data model - defines Action and PieItem types
/// This module contains pure data structures with no dependencies on UI or async runtime

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Command(String),
    Activate {
        service: String,
        path: String,
        menu_path: String,
    },
    Context {
        service: String,
        path: String,
        menu_path: String,
    },
    DbusSignal {
        service: String,
        path: String,
        id: i32,
    },
    None,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PieItem {
    pub label: String,
    pub icon: String,
    pub action: Action,
    pub children: Vec<PieItem>,
    pub item_type: Option<String>,
    pub tray_id: Option<String>,
}
