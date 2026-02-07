# Waypie


[![License](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange)](https://www.rust-lang.org)
[![Wayland](https://img.shields.io/badge/Protocol-Wayland-green)](https://wayland.freedesktop.org/)
[![Arch Linux](https://img.shields.io/badge/Arch-btw-1793d1)](https://archlinux.org)

> A radial menu for Wayland<br>

![Waypie Demo](img/demo.png)

> **Status: Experimental Prototype / Demo**<br>
> This project is currently in a **proof-of-concept** stage. While functional, it contains experimental code and incomplete features (especially regarding system tray integration).

#### Current features
- Cursor Auto Centering
- Basic System Tray (SNI) Host [Experimental]
- Interactive 2-Level Radial Menu (Categories & Sub-actions)
- Custom Shell Script & Command Execution
- Wayland Native (Layer Shell)
- Real-time Clock
- Hot-Reloading Configuration

#### Configuration and Styling

Waypie is configurable via `TOML`

[See the example configuration](https://github.com/vkkkv/waypie/tree/main/examples/config.toml)

### Installation

#### Building from source

```bash
git clone https://github.com/vkkkv/waypie
cd waypie
cargo build --release
./target/release/waypie
```

**Dependencies**

```
gtk4
gtk4-layer-shell
wayland
wayland-protocols
libdbusmenu-glib
```

## License

Licensed under GPLv3
