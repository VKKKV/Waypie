use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

static DEBUG_ENABLED: OnceLock<bool> = OnceLock::new();

static UI_UPDATE_SIGNALS: AtomicU64 = AtomicU64::new(0);
static UI_CONFIG_APPLIES: AtomicU64 = AtomicU64::new(0);
static UI_ITEMS_APPLIES: AtomicU64 = AtomicU64::new(0);
static RADIAL_DRAWS: AtomicU64 = AtomicU64::new(0);

fn debug_enabled() -> bool {
    *DEBUG_ENABLED.get_or_init(|| {
        std::env::var("WAYPIE_DEBUG")
            .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
            .unwrap_or(false)
    })
}

fn log_counter(name: &str, value: u64) {
    if debug_enabled() {
        println!("[WAYPIE_DEBUG] {}={}", name, value);
    }
}

pub fn incr_ui_update_signals() {
    let value = UI_UPDATE_SIGNALS.fetch_add(1, Ordering::Relaxed) + 1;
    log_counter("ui_update_signals", value);
}

pub fn incr_ui_config_applies() {
    let value = UI_CONFIG_APPLIES.fetch_add(1, Ordering::Relaxed) + 1;
    log_counter("ui_config_applies", value);
}

pub fn incr_ui_items_applies() {
    let value = UI_ITEMS_APPLIES.fetch_add(1, Ordering::Relaxed) + 1;
    log_counter("ui_items_applies", value);
}

pub fn incr_radial_draws() {
    let value = RADIAL_DRAWS.fetch_add(1, Ordering::Relaxed) + 1;
    log_counter("radial_draws", value);
}
