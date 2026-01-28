# Waypie Project Overview

**Waypie** is a unified Rust-based desktop utility **for Arch Linux on Wayland** (supports Hyprland, Niri, GNOME, KDE, Sway, and other Wayland compositors). It leverages `gtk4`, `gtk4-layer-shell`, and `ksni` to provide a seamless Wayland-native visual experience.

## Optimized Structure

The project is consolidated into a single binary (`waypie`) with modular source code:

```
waypie/
├── Cargo.toml
└── src/
    ├── main.rs      # Entry point & Argument parsing
    ├── hud/         # HUD & Dashboard UI Logic (GTK4)
    │   └── mod.rs
    ├── tray/        # System Tray Logic (KSNI)
    │   └── mod.rs
    ├── config.rs    # Configuration Management
    └── utils.rs     # Common utilities
```

## Modes & Usage

The `waypie` binary supports different modes via command-line arguments:

1.  **Dashboard Mode (Default)**
    *   **Command:** `waypie`
    *   **Behavior:** Launches the persistent desktop radial wheel (HUD) AND the system tray icon.
    *   **Visuals:** Interactive radial interface showing time, date, volume, and app launcher rings.

2. **Daemon Mode**
    *   **Command:** `waypie daemon`
    *   **Behavior:** Launches the system tray icon only in the background.
    *   **Visuals:** Persistent tray icon with system tray menu items.

## Building

```bash
cargo build --release
```

The binary will be at `target/release/waypie`.

## Key Features

*   **Interactive Radial Wheel:** Central hub displays time, date, and volume with visual arc indicator (blue normally, red if > 80%).
*   **Hover Effects:** Visual feedback when hovering over center hub or outer ring segments.
*   **Customizable Ring Segments:** Configurable app launcher items arranged in a circle.
*   **System Tray Integration:** Supports recursive submenus in tray menu.
*   **Hyprland Native:** Built with `gtk4-layer-shell` for Wayland layer shell protocol integration as an overlay layer.
*   **Configurable Actions:** Center hub supports click and scroll handlers for volume control and custom commands.

## Configuration

### 1. Application Config (`~/.config/waypie/config.toml`)

Configures UI appearance, system tray menu items, and center hub actions.

```toml
icon = "archlinux-logo"

# Radial ring launcher items (clockwise from 12 o'clock)
[[items]]
label = "Terminal"
script = "ghostty"

[[items]]
label = "Browser"
script = "firefox"

# Optional: Submenu support
[[items]]
label = "Power"
[[items.items]]
label = "Shutdown"
script = "systemctl poweroff"
```

### 2. UI Customization (Optional)

```toml
[ui]
refresh_rate_ms = 200          # Redraw frequency
width = 400                     # Window width
height = 400                    # Window height
outer_radius = 180.0            # Ring segment radius
tray_inner_radius = 110.0       # Inner ring radius
vol_radius = 95.0               # Volume arc radius
font_family = "Sans"
hover_mode = "highlight"        # Visual hover effect

[ui.colors]
# Format: [R, G, B] or [R, G, B, A] (0.0–1.0 range)
background = [0.1, 0.1, 0.1, 0.9]
volume_track = [0.3, 0.3, 0.3, 0.5]
volume_arc = [0.09, 0.57, 0.82]
volume_warning = [0.8, 0.2, 0.2]
text = [1.0, 1.0, 1.0]
tray_even = [0.15, 0.15, 0.15, 0.9]
tray_odd = [0.2, 0.2, 0.2, 0.9]
hover_overlay = [1.0, 1.0, 1.0, 0.1]

[actions]
# Commands triggered by center hub interactions
left_click = "pavucontrol"      # Default: open volume control
right_click = ""                # Optional: right-click command
scroll_up = "pamixer -i 5"      # Default: increase volume 5%
scroll_down = "pamixer -d 5"    # Default: decrease volume 5%
```

## Architecture Notes

- **Concurrency:** Config loaded once at startup; tray runs async in background, HUD blocks on GTK event loop
- **State Management:** Hover state and active submenu tracked via `Rc<RefCell<>>` to prevent memory cycles
- **Rendering:** Single draw callback per refresh cycle; Cairo drawing backend via GTK4
- **Error Handling:** Tray dbus failures silently caught (visible with `WAYPIE_DEBUG=1`); config errors fatal
- **Input Dispatch:** Click, scroll, and motion events routed based on hover target (Center, RingItem, OuterRingItem)
- **Wayland-Only:** No X11 compatibility layer; all APIs are Wayland-native
- **Multi-Compositor Support:** Auto-detects Wayland compositor and uses appropriate cursor APIs
  - Hyprland: Uses `hyprctl cursorpos` for position query, `hyprctl dispatch movecursor` for movement
  - Other compositors: Falls back to centering wheel on screen (graceful degradation)
