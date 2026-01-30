# Waypie

## Features

*   **Qt/QML Powered:** Smooth, hardware-accelerated UI with modern transparency and animations.
*   **Interactive Radial Wheel:** Central hub for time, date, and volume.
*   **Dynamic System Tray:** Monitors and displays tray items from other apps via DBus.
*   **Highly Configurable:** Customize colors, sizes, and actions (Work in progress for Qt port).
*   **Universal Wayland Support:** Works on Hyprland, Niri, GNOME, KDE, Sway.

## Installation

### Prerequisites

*   **Qt 6** (Core, Gui, Qml, Quick)
*   **CMake** (Required for CXX-Qt build)

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

