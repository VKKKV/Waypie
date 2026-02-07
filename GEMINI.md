# Waypie Project Overview

**Waypie** is a high-performance, Rust-based desktop HUD and utility **for Arch Linux on Wayland** (Sway/Hyprland). It provides an interactive 2-level radial menu with smooth animations and deep system integration.

## Optimized Structure

The project follows a modular architecture with a clear separation between the GTK4 UI thread, Wayland protocol interactions, and background asynchronous tasks:

```
waypie/
├── Cargo.toml
└── src/
    ├── main.rs           # Entry point, Global Runtime, and module declarations
    ├── ui.rs             # GTK4 UI construction and event loop management
    ├── cursor.rs         # Wayland Virtual Pointer logic (wlr-protocols)
    ├── hud/              # HUD & UI Components
    │   ├── mod.rs        # Module definitions
    │   ├── radial.rs     # Legacy/Alternative radial implementation
    │   └── radial_menu/  # Main Radial Menu Widget (GObject Subclassing)
    │       ├── mod.rs    # Public API, hit-detection, and click handling
    │       └── imp.rs    # Private implementation and Cairo drawing logic
    ├── notifier_host/    # SystemNotifierHost implementation (SNI Watcher)
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── host.rs
    │       ├── watcher.rs
    │       └── ...
    ├── dbus_menu.rs      # Manual DBusMenu client (com.canonical.dbusmenu)
    ├── config.rs         # Configuration Management (TOML) & Hot-Reloading (notify)
    ├── utils.rs          # Async command execution & shell parsing (shlex)
    ├── sni_watcher.rs    # Hybrid SNI Watcher (Server/Client) & Path Discovery
    └── color.rs          # Color parsing and utility functions
```

## Core Technologies

*   **UI:** `gtk4` with `cairo-rs` for high-performance custom drawing.
*   **Wayland:** 
    *   `gtk4-layer-shell` for overlay positioning and shell integration.
    *   `wayland-client` & `wayland-protocols-wlr` for virtual pointer manipulation (cursor centering).
*   **Async:** `tokio` multi-threaded runtime (**Global Static**) for file watching and DBus communication.
*   **Interop:** `async-channel` and `glib::MainContext::channel` for safe thread communication.
*   **Config:** `toml` for configuration with `notify` for real-time hot-reloading.
*   **DBus:** `zbus` (v4) for StatusNotifierItem (SNI) implementation.
*   **XML:** `quick-xml` for DBus introspection and path auto-discovery.

## Key Features

*   **Interactive 2-Level Radial Menu:** 
    *   **Inner Ring (Ring 1):** Parent categories that expand on hover.
    *   **Outer Ring (Ring 2):** Sub-actions/Children that slide out smoothly.
    *   **Center Display:** Real-time clock and info display in the middle of the wheel.
    *   **Dead Zone Logic:** Radius-based hit detection (center dead zone) to prevent accidental triggers.
    *   **Efficiency Logic:** 100ms hover delay before expansion to prevent accidental flickering.
*   **Cursor Auto-Centering:** Automatically teleports the cursor to the center of the HUD upon activation using `wlr-virtual-pointer-v1`, ensuring immediate accessibility for keyboard-centric workflows.
*   **Advanced System Tray (SNI) Support:**
    *   **Hybrid Watcher:** Acts as a `StatusNotifierWatcher` server on environments that lack one (like Hyprland) or a client on environments that have one (like KDE).
    *   **Event-Driven:** Reactive updates using DBus signals instead of polling.
    *   **Path Auto-Discovery:** Uses recursive DBus introspection to find the correct object path for apps with non-standard tray implementations (e.g., Electron apps).
    *   **Integrated DBusMenu Client:** Uses `dbusmenu-glib` asynchronously to fetch and map application context menus (e.g. `nm-applet`) directly into the radial menu as sub-rings.
        *   **AboutToShow Support:** Explicitly triggers the `AboutToShow` DBus method to ensure dynamic items are populated before fetching.
        *   **Signal Debouncing:** Implements a 75ms debounce timer for `layout-updated` signals to handle applications that update their menu structure in multiple asynchronous bursts (like Fcitx5).
    *   **Interactive Tray Icons:** Support for left-click (activate) and right-click (context menu fetch and display). Actions are routed back via `com.canonical.dbusmenu.Event` signals.
*   **"Hyprland-Style" Animations:** 
    *   Smooth Linear Interpolation (Lerp) for ring expansions and fade-ins.
    *   Configurable animation speeds and progress tracking.
*   **Robust Configuration:**
    *   Fully customizable HUD dimensions, radii, and **colors** (backgrounds, text, stroke, hover/active states) via `config.toml`.
    *   Automatic default config generation with test items (Web, Terminal, Files, Tray).
    *   Hot-reloading supported: edit `~/.config/waypie/config.toml` and see changes instantly.
*   **System Integration:**
    *   Native KSNI support for system tray icons.
    *   Shell-aware command execution with detached process spawning.

## Building

```bash
cargo build --release
```

The binary will be at `target/release/waypie`. Performance is optimized via LTO and symbol stripping in the release profile.

## Usage

Launch the HUD:
```bash
waypie
```
*   **Center:** View the time or click to close the HUD.
*   **Inner Ring:** Hover to expand submenus.
*   **Outer Ring:** Click to execute actions.
*   **Tray Item:** Left-click to activate, right-click for context menu.
*   **Escape:** Close the HUD immediately.