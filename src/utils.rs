use crate::color::{Color3, Color4};
use std::process::Command;
use wayland_client::{
    protocol::{wl_registry, wl_seat},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1, zwlr_virtual_pointer_v1,
};

pub fn execute_command(cmd: &str) {
    let status = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .spawn();
    
    if let Err(e) = status {
        eprintln!("Failed to execute command '{}': {}", cmd, e);
    }
}

struct AppData {
    seat: Option<wl_seat::WlSeat>,
    manager: Option<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            if interface == "wl_seat" {
                state.seat = Some(registry.bind(name, 1, qh, ()));
            } else if interface == "zwlr_virtual_pointer_manager_v1" {
                state.manager = Some(registry.bind(name, 1, qh, ()));
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for AppData {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1, ()> for AppData {
    fn event(
        _: &mut Self,
        _: &zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
        _: zwlr_virtual_pointer_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1, ()> for AppData {
    fn event(
        _: &mut Self,
        _: &zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
        _: zwlr_virtual_pointer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {}
}

pub fn center_cursor() {
    // Attempt to connect to Wayland display
    let conn = match Connection::connect_to_env() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to connect to Wayland for cursor warping: {}", e);
            return;
        }
    };

    let display = conn.display();
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let mut data = AppData {
        seat: None,
        manager: None,
    };

    let _registry = display.get_registry(&qh, ());

    // Roundtrip to get globals
    if let Err(e) = event_queue.roundtrip(&mut data) {
        eprintln!("Wayland roundtrip failed: {}", e);
        return;
    }

    if let (Some(seat), Some(manager)) = (data.seat.as_ref(), data.manager.as_ref()) {
        let virtual_pointer = manager.create_virtual_pointer(Some(seat), &qh, ());
        
        // Move to absolute center (normalized 0.5, 0.5)
        // We define an arbitrary extent (e.g. 1000x1000) and move to 500x500.
        // This centers it on the total output bounding box.
        let extent = 1000;
        let center = 500;
        
        virtual_pointer.motion_absolute(0, center, center, extent, extent);
        virtual_pointer.frame();
        
        // Sync to ensure event is sent
        let _ = event_queue.roundtrip(&mut data);
        
        println!("Emitted virtual pointer absolute motion to center.");
    } else {
        eprintln!("Required globals (wl_seat or zwlr_virtual_pointer_manager_v1) not found.");
    }
}

/// Convert hex color (0xRRGGBB) to RGB tuple with normalized values (0.0-1.0)
pub const fn hex_to_rgb(hex: u32) -> Color3 {
    let r = ((hex >> 16) & 0xFF) as f64 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f64 / 255.0;
    let b = (hex & 0xFF) as f64 / 255.0;
    (r, g, b)
}

/// Convert hex color (0xRRGGBBAA) to RGBA tuple with normalized values (0.0-1.0)
pub const fn hex_to_rgba(hex: u32) -> Color4 {
    let r = ((hex >> 24) & 0xFF) as f64 / 255.0;
    let g = ((hex >> 16) & 0xFF) as f64 / 255.0;
    let b = ((hex >> 8) & 0xFF) as f64 / 255.0;
    let a = (hex & 0xFF) as f64 / 255.0;
    (r, g, b, a)
}