# Waypie Project Overview

**Waypie** is a unified Rust-based desktop utility **for Arch Linux on Wayland**. It leverages `gtk4`, `gtk4-layer-shell`, and `ksni` to provide a seamless Wayland-native visual experience.

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
*   **Configurable Actions:** Center hub supports click and scroll handlers for volume control and custom commands.

