# Waypie Copilot Instructions

## Project Overview

**Waypie** is a unified Rust-based desktop utility for **Arch Linux on Wayland** (Hyprland, Niri, GNOME, KDE, Sway, etc.). It provides a radial command center (HUD) and system tray integration using GTK4, gtk4-layer-shell, and KSNI. The binary runs in two modes:

1. **Default (Radial Wheel)**: Interactive circular UI showing time, date, volume, and app launcher rings
2. **Daemon Mode** (`waypie daemon`): Background tray icon only

## Architecture

The codebase is organized as a single binary with modular components:

- **main.rs**: Entry point handling CLI argument parsing (default mode vs daemon mode)
- **hud/mod.rs** (~23KB): GTK4-based visual layer rendering the radial wheel, handling input (click, scroll, hover), and drawing core visual elements (center hub with time/date/volume, ring segments for app launching)
- **tray/mod.rs**: KSNI-based system tray implementation with recursive submenu support
- **config.rs**: TOML configuration deserialization (UI settings, colors, app menu items, center hub actions)
- **utils.rs**: Command execution utility

### Key Design Pattern

- Config is loaded once at startup via `config::load()` and passed into both tray and HUD modules
- Both modules run async concurrently (tray spawned as background task, HUD blocks on GTK event loop)
- HUD uses GTK drawing area with Cairo backend for rendering; hover state tracked in RefCell<HoverTarget> enum
- Tray uses KSNI's dbus interface; tray errors are silent with optional debug output via `WAYPIE_DEBUG` env var

## Build & Development

### Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release
```

Binary location: `target/release/waypie` or `target/debug/waypie`

### Run Locally

```bash
# Radial wheel mode
cargo run

# Daemon mode
cargo run -- daemon

# With debug output for tray errors
WAYPIE_DEBUG=1 cargo run
```

### Check for Issues

```bash
# Linting & formatting check
cargo clippy

# Format check (does not modify)
cargo fmt -- --check

# Fix formatting
cargo fmt
```

**Note**: No automated tests exist in the codebase. Manual testing is required.

## Configuration

User config: `~/.config/waypie/config.toml`

**Required field**: `icon` (icon name from system theme)

**Sections**:
- `[[items]]`: Menu items with `label` and optional `script` (recursive submenus supported via nested `items`)
- `[ui]`: Refresh rate, dimensions, radii, font, hover mode, color overrides
- `[ui.colors]`: RGBA as floats 0.0–1.0
- `[actions]`: Click/scroll handlers for center hub (left_click, right_click, scroll_up, scroll_down)

Defaults are applied via `#[serde(default = "...")]` pattern in config.rs.

## Key Conventions

### Code Organization
- Each module has a single `pub fn run()` or `pub async fn run()` entry point
- Config is immutable after load; state changes (hover, submenu) managed in HUD via Rc<RefCell<>>
- GTK signal handlers use closures capturing necessary state through weak refs to prevent cycles

### Hover State & Input Handling
HUD tracks hover target as enum: `None | Center | RingItem(index) | OuterRingItem(index)`
- Motion updates hover state; drawing uses it to apply visual effects (highlight overlay, text emphasis)
- Click/scroll dispatches based on hover target
- Submenu toggle on ring item click; center hub click triggers actions (defaults: left=pavucontrol, right=none)

### Drawing & Rendering
- Radii and geometry defined in config; center positioned at window center
- Inner ring: volume arc visualization; outer ring: app launcher segments
- All drawing in one draw_func callback; refresh triggered every `refresh_rate_ms`
- Colors support alpha; background color controls transparency

### Dependencies & Integration
- **gtk4**: Window, drawing, event handling (Wayland-native)
- **gtk4-layer-shell**: Wayland layer shell protocol for overlay integration
- **ksni**: D-Bus system tray (Wayland-compatible)
- **chrono**: Time/date formatting
- **tokio**: Async runtime (required for tray dbus)
- **cairo**: Drawing backend (via gtk4)

### Hyprland Integration
- **Multi-compositor support**: Auto-detects Wayland compositor (Hyprland, Sway, GNOME, KDE, etc.)
- **Hyprland cursor control**: `hyprctl cursorpos` (position query), `hyprctl dispatch movecursor` (movement)
- **Fallback behavior**: Centers wheel on screen when cursor position unavailable (non-Hyprland compositors)

### Error Handling
- Tray spawn failures silently caught (logged to stderr if WAYPIE_DEBUG=1)
- Config load failures will crash (file not found, parse error)
- Command execution errors logged to stderr

## Wayland-Only Constraints

This project is **Wayland-only** (supports all Wayland compositors):

- All APIs must be Wayland-compatible (gtk4-layer-shell, D-Bus for tray, Wayland protocols)
- Do NOT add X11 dependencies or fallbacks (e.g., xdotool, X11 libraries)
- **Multi-compositor support**: Code detects and adapts to different Wayland compositors (Hyprland, Sway, GNOME, KDE, Niri, etc.)
- Cursor positioning uses compositor-specific methods where available, with graceful fallback to screen center
- Layer shell is mandatory for overlay positioning

## Testing Checklist for Contributors

When adding features or fixing bugs:

1. Build in debug and release modes: `cargo build && cargo build --release`
2. Run clippy: `cargo clippy` (no warnings expected)
3. Format code: `cargo fmt`
4. Test manual scenarios:
   - Launch in radial mode: confirm rendering, hover effects, clicks work
   - Launch in daemon mode: confirm tray icon appears and menu responds
   - ESC key exits cleanly
   - Config changes (colors, refresh rate, items) applied on next launch
   - Center hub actions trigger scripts correctly
   - Submenu nesting works in tray
