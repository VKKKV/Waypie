# Waypie

**Waypie** is a unified Radial Command Center for **Arch Linux on Wayland**. It provides a futuristic, controller-friendly, and mouse-driven circular interface for system controls and app launching.

### 🌍 Multi-Compositor Support

Works on all Wayland compositors:
- **Hyprland** ✅ Full cursor positioning support
- **Niri** ✅ Screen-centered positioning
- **GNOME** ✅ Screen-centered positioning
- **KDE Plasma** ✅ Screen-centered positioning
- **Sway** ✅ Screen-centered positioning
- **Any Wayland compositor** ✅ Graceful fallback

## Features

*   **Interactive Radial Wheel:** Central hub for time, date, and volume, surrounded by a customizable ring of application launchers.
*   **Hover Effects:** Visual feedback when hovering over the center hub or ring segments (highlight).
*   **Highly Configurable:** Customize colors, sizes, radii, polling rates, and mouse actions.
*   **System Tray Integration:** Optional background daemon for tray icon support.
*   **Universal Wayland Support:** Works on Hyprland, Niri, GNOME, KDE, Sway, and any Wayland compositor.
*   **Smart Cursor Positioning:** Opens wheel at cursor on Hyprland; gracefully centers on other compositors.

## Installation

### 1. Build from Source

```bash
cargo build --release
```

The binary will be located at `target/release/waypie`.

### 2. Configuration

Create a config file at `~/.config/waypie/config.toml`.

#### Basic Configuration

```toml
icon = "archlinux-logo"

# Radial Slices (Clockwise starting from 12 o'clock)
[[items]]
label = "Terminal"
script = "ghostty"

[[items]]
label = "Web"
script = "firefox"

[[items]]
label = "Files"
script = "thunar"

[[items]]
label = "Power"
script = "wlogout"
```

#### Advanced UI Customization (Optional)

```toml
[ui]
refresh_rate_ms = 200
width = 400
height = 400
outer_radius = 180.0
tray_inner_radius = 110.0
vol_radius = 95.0
font_family = "Sans"
hover_mode = "highlight"

[ui.colors]
# Format: [R, G, B] or [R, G, B, A] (0.0 to 1.0)
background = [0.1, 0.1, 0.1, 0.9]
volume_track = [0.3, 0.3, 0.3, 0.5]
volume_arc = [0.09, 0.57, 0.82]
volume_warning = [0.8, 0.2, 0.2]
text = [1.0, 1.0, 1.0]
tray_even = [0.15, 0.15, 0.15, 0.9]
tray_odd = [0.2, 0.2, 0.2, 0.9]
hover_overlay = [1.0, 1.0, 1.0, 0.1]

[actions]
# Commands for interactions with the Central Hub
# Defaults:
left_click = "pwvucontrol"
right_click = (none)
scroll_up = "pamixer -i 5"
scroll_down = "pamixer -d 5"

# Example: Mute on click, Open mixer on right click
left_click = "pamixer -t"
right_click = "pwvucontrol"
```

## Usage

### Radial Wheel (Default)

Launch the interactive radial menu:

```bash
waypie
```

*   **Center:** Displays Time, Date, and Volume.
*   **Inner Ring:** Visual Volume Arc (Blue normally, Red if > 80%).
*   **Outer Ring:** Clickable segments to launch configured applications.
*   **Exit:** Press `ESC` to close the wheel.

### Background Service

Run the system tray service in the background:

```bash
waypie daemon
```

## Hyprland Integration

Add the following to your `~/.config/hypr/hyprland.conf`:

```conf
# Path to your compiled binary
$waypie = $HOME/path/to/waypie/target/release/waypie

# Open the Radial Command Wheel (SUPER + W)
bind = SUPER, W, exec, $waypie

# Autostart the background tray service (Optional)
exec-once = $waypie daemon
```

## Other Wayland Compositor Integration

### GNOME (Settings or Extensions)
```bash
waypie              # Run from Activities or command line
waypie daemon       # Optional background tray
```

### KDE Plasma (Global Shortcuts)
1. System Settings → Shortcuts → Custom Shortcuts
2. Create new shortcut for `waypie`
3. Or run via application menu

### Sway (~/.config/sway/config)
```conf
bindsym $mod+w exec waypie
```

### Niri
```bash
waypie              # Launch from menu or terminal
waypie daemon       # Optional background service
```
