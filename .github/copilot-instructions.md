# Copilot Instructions for Waypie

Waypie is a Rust-based desktop utility for Wayland compositors (Hyprland, Niri, GNOME, KDE, etc.). It provides an interactive radial menu with time/date/volume display, app launcher rings, and system tray integration.

## Build & Test

### Build
```bash
cargo build --release
```
Binary output: `target/release/waypie`

### Check Compilation
```bash
cargo check
```

### Linting
```bash
cargo clippy
```
Note: Currently has warnings for `too_many_arguments` in `draw_radial_wheel()` and `collapsible_if` in hover logic. These are known and lower-priority refactoring candidates.

### Running
- **Default mode (radial menu + tray):** `./target/release/waypie`
- **Daemon mode (tray only):** `./target/release/waypie daemon`

### Debug
Set `WAYPIE_DEBUG=1` environment variable to enable debug output for SNI watcher errors:
```bash
WAYPIE_DEBUG=1 ./target/release/waypie
```

## Architecture

### Single Binary, Modular Design
Waypie consolidates all functionality into one binary (`waypie`) with modular source files under `src/`:

**Core Modules:**
- **`main.rs`** – Entry point; parses args to dispatch between radial wheel (default) and daemon mode
- **`hud/mod.rs`** – GTK4 drawing logic for the radial wheel UI (center hub + ring segments + tray items display)
- **`tray/mod.rs`** – KSNI (KDE System Notifier Interface) implementation for system tray icon and menu
- **`config.rs`** – TOML config parsing; defines all config structs with defaults; loaded from `~/.config/waypie/config.toml`
- **`sni_watcher.rs`** – Watches D-Bus for SNI (System Notifier Items) and provides tray app metadata
- **`utils.rs`** – Common utilities (e.g., `execute_command()` for running scripts)

### Key Flow
1. **Default mode:** `main.rs` spawns SNI watcher on separate thread, starts tray service (`tray::run`), then runs HUD UI (`hud::run`)
2. **Daemon mode:** Only tray service runs in background
3. **HUD lifecycle:** GTK4 app window is fullscreen layer-shell overlay; ESC exits

### State Management
- **`HudState` enum** – Tracks whether idle (normal wheel), tray ring active, or context menu active
- **`HoverTarget` enum** – Detects what's under cursor (center, ring item, tray button, etc.)
- **Colors & UI settings** – Fully configurable via TOML; defaults provided in `config.rs`

## Key Conventions

### Configuration Pattern
All configs use `serde` with `#[serde(default = "fn_name")]` pattern for optional fields. Default functions are defined at top of `config.rs`:
```rust
fn default_icon() -> String { "archlinux-logo".to_string() }
pub icon: String,  // Will use default if not in TOML
```
This allows users to omit settings and get sensible defaults.

### Color Representation
Colors use tuples: `(r, g, b)` for RGB or `(r, g, b, a)` for RGBA with values 0.0–1.0 (not 0–255).

### Command Execution
Use `execute_command()` from `utils.rs` to run arbitrary shell commands (scripts, app launches, volume control). This handles shell expansion.

### Cairo Drawing Context
HUD rendering uses GTK4's Cairo context. Drawing functions take `&Context` and draw via methods like:
```rust
context.set_source_rgba(r, g, b, a);
context.arc(cx, cy, radius, start_angle, end_angle);
context.stroke();
```
Angles use radians; 0 radians = 12 o'clock, increases clockwise.

### Async Runtime
- Tray service runs asynchronously via Tokio (`pub async fn run()`)
- SNI watcher also async but spawned on separate thread with its own tokio runtime to avoid blocking HUD
- Main HUD UI is synchronous (GTK4 mainloop)

### Configuration Loading
`config::load()` uses XDG base directories to find config at `~/.config/waypie/config.toml`. Falls back to hardcoded defaults with sample menu items (Terminal, Browser) if file doesn't exist.

### Exit Behavior
ESC key bound to `std::process::exit(0)` in HUD. Exit action in tray menu also calls `std::process::exit(0)`.

## Common Tasks

### Adding a New Config Option
1. Add field to appropriate struct in `config.rs` (e.g., `UiConfig`, `ColorConfig`, `ActionConfig`)
2. Define default function at top: `fn default_my_option() -> Type { ... }`
3. Use `#[serde(default = "default_my_option")]`
4. Use the value in HUD or tray code

### Adding a New Ring Menu Item Type
Ring items are configurable via `items: Vec<MenuItemConfig>` in config. Submenu support is already implemented; recursive menu items in tray work via `build_menu_items()` recursion in `tray/mod.rs`.

### Debugging SNI Issues
Enable `WAYPIE_DEBUG=1` to see SNI watcher errors. SNI watcher spawns on separate thread, so errors won't block HUD but will print to stderr.

### Testing Visual Changes
Changes to colors, sizes, or drawing logic are in `hud/mod.rs` inside `draw_radial_wheel()`. After building, run `./target/release/waypie` to test interactively.
