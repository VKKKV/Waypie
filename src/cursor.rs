use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_registry, wl_seat},
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1, zwlr_virtual_pointer_v1,
};

struct AppData {
    seat: Option<wl_seat::WlSeat>,
    manager: Option<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1>,
}

// Fix 1: Correct UserData type
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for AppData {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // We don't need to do anything here, the helper maintains the list.
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
    ) {
    }
}

impl Dispatch<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1, ()> for AppData {
    fn event(
        _: &mut Self,
        _: &zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
        _: zwlr_virtual_pointer_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1, ()> for AppData {
    fn event(
        _: &mut Self,
        _: &zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
        _: zwlr_virtual_pointer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

pub fn move_cursor_to_center(width: u32, height: u32) {
    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to Wayland: {}", e);
            return;
        }
    };

    let (globals, mut event_queue) = match registry_queue_init::<AppData>(&conn) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to init registry queue: {}", e);
            return;
        }
    };

    let qh = event_queue.handle();
    let registry = globals.registry();

    // Bind Seat (first available)
    let seat = globals.contents().with_list(|list| {
        list.iter()
            .find(|g| g.interface == wl_seat::WlSeat::interface().name && g.version >= 1)
            .map(|g| registry.bind::<wl_seat::WlSeat, _, _>(g.name, 1, &qh, ()))
    });

    // Bind Virtual Pointer Manager
    let manager = globals.contents().with_list(|list| {
        list.iter()
            .find(|g| {
                g.interface
                    == zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1::interface()
                        .name
                    && g.version >= 1
            })
            .map(|g| {
                registry.bind::<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1, _, _>(
                    g.name,
                    1,
                    &qh,
                    (),
                )
            })
    });

    let mut state = AppData { seat, manager };

    // Check if we got everything
    if state.seat.is_none() {
        eprintln!("Waypie: Could not find wl_seat global.");
        return;
    }
    if state.manager.is_none() {
        eprintln!("Waypie: Could not find zwlr_virtual_pointer_manager_v1 global. Is your compositor compatible with wlr-protocols?");
        return;
    }

    let _ = event_queue.roundtrip(&mut state);

    // Fix 3: Wrap seat in Some()
    let pointer = state.manager.as_ref().unwrap().create_virtual_pointer(
        Some(state.seat.as_ref().unwrap()),
        &qh,
        (),
    );

    // Center logic
    let x = width / 2;
    let y = height / 2;

    // motion_absolute(time, x, y, extent_x, extent_y)
    pointer.motion_absolute(0, x, y, width, height);
    pointer.frame();

    // Flush and Roundtrip
    let _ = conn.flush();
    let _ = event_queue.roundtrip(&mut state);

    // Cleanup
    pointer.destroy();
    let _ = conn.flush();
}
