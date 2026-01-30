# Waypie Project Overview

**Waypie** is a high-performance, unified desktop utility engineered exclusively for **Arch Linux on Wayland**. It is built with a **Pure Rust** architecture, leveraging **Iced** and **`iced_layershell`** to deliver a native, resource-efficient, and visually cohesive user experience.

## Architectural Highlights

*   **Pure Rust & Wayland Native:** Eliminates legacy bindings (Qt/CXX) in favor of a type-safe, memory-safe Rust implementation.
*   **Wayland Layer Shell:** Utilizes the `iced_layershell` crate to render the HUD as a genuine Wayland Overlay. This ensures proper z-ordering (floating above windows), input handling, and transparency without hacking X11 hints.
*   **Declarative UI:** powered by **Iced**, allowing for a reactive, elm-architecture-inspired interface definition.
*   **Unified Binary:** consolidates the Dashboard (HUD) and System Tray logic into a single optimized executable.

## Project Structure

```
waypie/
├── Cargo.toml       # Dependencies (iced, iced_layershell, ksni, tokio)
└── src/
    ├── main.rs      # Entry point: Initializes Iced Layershell or Daemon mode
    ├── hud/         # Iced Layershell UI implementation
    │   ├── mod.rs
    │   └── app.rs   # State management and View logic
    ├── tray/        # System Tray logic (KSNI)
    │   └── mod.rs
    ├── config.rs    # Configuration parsing (TOML)
    ├── sni_watcher.rs # DBus StatusNotifierItem discovery
    └── utils.rs     # Shared utilities
```

## Modes & Operation

The `waypie` binary operates in two primary modes:

1.  **Dashboard Mode (HUD)**
    *   **Command:** `waypie`
    *   **Mechanism:** Initializes a Wayland Layer Shell surface via `iced_layershell`.
    *   **Features:** Displays the radial menu, time, date, and volume controls. Supports keyboard navigation and transparency.

2.  **Daemon Mode**
    *   **Command:** `waypie daemon`
    *   **Mechanism:** Runs a lightweight background process using `tokio` and `ksni`.
    *   **Features:** Hosts the System Tray icon and handles DBus communication for SNI discovery.

## Build Requirements

*   **Rust:** Edition 2024 (stable)
*   **System Libraries:** `wayland-client`, `libxkbcommon`, `vulkan` (or `opengl`) drivers.

```bash
cargo build --release
```

**Artifact:** `target/release/waypie`