use gtk4::{Application, ApplicationWindow, CssProvider};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use crate::config::UiConfig;

pub fn build_window(app: &Application, config: &UiConfig) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(config.width)
        .default_height(config.height)
        .build();

    // Layer Shell
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Bottom, true);

    // Transparency
    let provider = CssProvider::new();
    provider.load_from_data("window { background-color: transparent; }");
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("No display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    window
}
