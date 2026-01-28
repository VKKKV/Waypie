# Waypie Project Overview

**Waypie** is a unified Rust-based desktop utility suite for **Arch Linux** on **Hyprland**. It leverages `gtk4`, `gtk4-layer-shell`, and `ksni` to provide a seamless visual experience.

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
    *   **Behavior:** Launches the persistent desktop clock/volume widget AND the system tray icon.
    *   **Visuals:** Persistent radial interface showing time, date, and volume.

2.  **HUD Mode (Transient)**
    *   **Command:** `waypie hud`
    *   **Behavior:** A transient volume overlay. Appears for 1.5 seconds then exits.
    *   **Visuals:** Simplified radial interface (Volume % only, Red warning if > 80%).

3.  **Tray Mode**
    *   **Command:** `waypie tray`
    *   **Behavior:** Runs only the system tray icon.
    *   **Use Case:** If you prefer a standalone tray without the dashboard.

## Building

```bash
cargo build --release
```

The binary will be at `target/release/waypie`.

## Configuration

### 1. Hyprland Integration (`~/.config/hypr/hyprland.conf`)

Update your keybindings to use the new subcommands:

```conf
$waypie = $HOME/path/to/waypie/target/release/waypie

# Volume HUD (Transient)
binde = , XF86AudioRaiseVolume, exec, pamixer -i 5 && $waypie hud
binde = , XF86AudioLowerVolume, exec, pamixer -d 5 && $waypie hud

# Toggle Tray (Optional)
bind = SUPER, T, exec, /path/to/scripts/tray-toggle.sh
```

### 2. Application Config (`~/.config/waypie/config.toml`)

Configures the Tray Menu items.

```toml
icon = "archlinux-logo"

[[items]]
label = "Terminal"
script = "kitty"

[[items]]
label = "Browser"
script = "firefox"
```