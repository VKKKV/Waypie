use crate::config::AppConfig;
use crate::utils::execute_command;
use iced::widget::canvas::{self, Canvas, Path, Stroke};
use iced::widget::{column, text, container, mouse_area};
use iced::{
    alignment, mouse, time, keyboard, Color, Element, Length, Rectangle, Subscription,
    Task, Theme, Event, Point, Padding,
};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::reexport::{Anchor, Layer, KeyboardInteractivity};
use iced_layershell::actions::LayershellCustomActionWithId;
use std::time::{Duration, Instant};

pub struct WaypieHud {
    time: String,
    date: String,
    volume: f32,
    volume_text: String,
    config: AppConfig,
    cursor_position: Option<Point>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick(Instant),
    LayerShellEvent(LayershellCustomActionWithId),
    LeftClick,
    RightClick,
    Scroll(mouse::ScrollDelta),
    Exit,
    CursorMoved(Point),
    TriggerCursor,
}

impl TryInto<LayershellCustomActionWithId> for Message {
    type Error = Message;

    fn try_into(self) -> Result<LayershellCustomActionWithId, Self::Error> {
        if let Message::LayerShellEvent(action) = self {
            Ok(action)
        } else {
            Err(self)
        }
    }
}

impl WaypieHud {
    fn new(config: AppConfig) -> (Self, Task<Message>) {
        let (t, d, v, vt) = Self::get_data();
        
        // Only center if NOT opening at cursor
        if !config.ui.open_at_cursor {
             crate::utils::center_cursor();
        }

        let initial_task = if config.ui.open_at_cursor {
            Task::perform(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }, |_| Message::TriggerCursor)
        } else {
            Task::none()
        };

        (
            Self {
                time: t,
                date: d,
                volume: v,
                volume_text: vt,
                config,
                cursor_position: None,
            },
            initial_task,
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(_) => {
                let (t, d, v, vt) = Self::get_data();
                self.time = t;
                self.date = d;
                self.volume = v;
                self.volume_text = vt;
            }
            Message::TriggerCursor => {
                return Task::perform(async {
                    tokio::task::spawn_blocking(|| {
                        crate::utils::trigger_cursor_event();
                    }).await.ok();
                }, |_| Message::Tick(Instant::now()));
            }
            Message::LeftClick => {
                if let Some(cmd) = &self.config.actions.left_click {
                    let cmd_to_run = if cmd == "pavol" { "pavucontrol" } else { cmd };
                    execute_command(cmd_to_run);
                }
            }
            Message::RightClick => {
                if let Some(cmd) = &self.config.actions.right_click {
                    execute_command(cmd);
                }
            }
            Message::Scroll(delta) => {
                match delta {
                    mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => {
                        if y > 0.0 {
                            if let Some(cmd) = &self.config.actions.scroll_up {
                                execute_command(cmd);
                            }
                        } else if y < 0.0 {
                            if let Some(cmd) = &self.config.actions.scroll_down {
                                execute_command(cmd);
                            }
                        }
                        return Task::perform(async {}, |_| Message::Tick(Instant::now()));
                    }
                }
            }
            Message::Exit => {
                std::process::exit(0);
            }
            Message::CursorMoved(point) => {
                if self.config.ui.open_at_cursor && self.cursor_position.is_none() {
                    self.cursor_position = Some(point);
                }
            }
            Message::LayerShellEvent(_) => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let width = self.config.ui.width as f32;
        let height = self.config.ui.height as f32;

        let ring = Canvas::new(Ring { volume: self.volume })
            .width(Length::Fixed(width))
            .height(Length::Fixed(height));

        let content = column![
            text(&self.time).size(42).color(Color::WHITE),
            text(&self.date).size(16).color(Color::from_rgb(0.66, 0.66, 0.66)),
            text(&self.volume_text).size(14).color(Color::from_rgb(0.53, 0.53, 0.53)),
        ]
        .spacing(5)
        .align_x(alignment::Horizontal::Center);

        let hud_stack = container(
            iced::widget::stack![
                ring,
                container(content).center_x(Length::Fill).center_y(Length::Fill)
            ]
        )
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .style(move |_theme| {
            container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.8).into()),
                border: iced::Border {
                    radius: (width / 2.0).into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            }
        });

        let clickable_hud = mouse_area(hud_stack)
            .on_press(Message::LeftClick)
            .on_right_press(Message::RightClick);

        let root_style = |_theme: &_| container::Style {
            background: Some(Color::TRANSPARENT.into()),
            ..Default::default()
        };

        if self.config.ui.open_at_cursor {
            if let Some(pos) = self.cursor_position {
                let left_pad = (pos.x - width / 2.0).max(0.0);
                let top_pad = (pos.y - height / 2.0).max(0.0);

                container(clickable_hud)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(Padding {
                        top: top_pad,
                        right: 0.0,
                        bottom: 0.0,
                        left: left_pad,
                    })
                    .align_x(alignment::Horizontal::Left)
                    .align_y(alignment::Vertical::Top)
                    .style(root_style)
                    .into()
            } else {
                container(text(""))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(root_style)
                    .into()
            }
        } else {
            container(clickable_hud)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(root_style)
                .into()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let time_sub = time::every(Duration::from_millis(1000)).map(Message::Tick);
        
        let io_sub = iced::event::listen_with(|event, _status, _window| {
            match event {
                Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                    Some(Message::Scroll(delta))
                }
                Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                    if key == keyboard::Key::Named(keyboard::key::Named::Escape) {
                        Some(Message::Exit)
                    } else {
                        None
                    }
                }
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                     Some(Message::CursorMoved(position))
                }
                _ => None
            }
        });

        Subscription::batch(vec![time_sub, io_sub])
    }

    fn get_data() -> (String, String, f32, String) {
        let now = chrono::Local::now();
        let t = now.format("%H:%M").to_string();
        let d = now.format("%a %d %b").to_string();
        let vol = get_volume();
        let vt = format!("Vol: {:.0}%", vol * 100.0);
        (t, d, vol, vt)
    }
}

struct Ring {
    volume: f32,
}

impl canvas::Program<Message> for Ring {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let center = frame.center();
        let radius = bounds.width.min(bounds.height) / 2.0;

        let path = Path::circle(center, radius - 1.0);
        frame.stroke(
            &path,
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.2))
                .with_width(2.0),
        );

        let vol_radius = radius - 20.0;
        let start_angle = -std::f32::consts::FRAC_PI_2;
        let end_angle = start_angle + self.volume * 2.0 * std::f32::consts::PI;

        let vol_path = Path::new(|p| {
            p.arc(canvas::path::Arc {
                center,
                radius: vol_radius,
                start_angle: iced::Radians(start_angle),
                end_angle: iced::Radians(end_angle),
            });
        });

        frame.stroke(
            &vol_path,
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.2))
                .with_width(8.0)
                .with_line_cap(canvas::LineCap::Round),
        );

        vec![frame.into_geometry()]
    }
}

fn get_volume() -> f32 {
    let output = std::process::Command::new("pamixer").arg("--get-volume").output();
    if let Ok(output) = output {
        let s = String::from_utf8_lossy(&output.stdout);
        return s.trim().parse().unwrap_or(0.0) / 100.0;
    }
    0.0
}

fn custom_theme() -> Theme {
    Theme::Custom(std::sync::Arc::new(iced::theme::Custom::new(
        "Transparent".into(),
        iced::theme::Palette {
            background: Color::TRANSPARENT,
            text: Color::WHITE,
            primary: Color::WHITE,
            success: Color::from_rgb(0.0, 1.0, 0.0),
            danger: Color::from_rgb(1.0, 0.0, 0.0),
            warning: Color::from_rgb(1.0, 1.0, 0.0),
        }
    )))
}

pub fn run(config: AppConfig) -> Result<(), iced_layershell::Error> {
    let width = config.ui.width as u32;
    let height = config.ui.height as u32;
    let open_at_cursor = config.ui.open_at_cursor;

    let (anchor, size) = if open_at_cursor {
        // Full screen, no explicit size (fill output)
        (Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right, None)
    } else {
        // Centered, explicit size
        (Anchor::empty(), Some((width, height)))
    };

    iced_layershell::application(
        move || WaypieHud::new(config.clone()), 
        "waypie", 
        WaypieHud::update, 
        WaypieHud::view
    )
    .subscription(WaypieHud::subscription)
    .theme(|_: &WaypieHud| custom_theme())
    .settings(Settings {
        layer_settings: LayerShellSettings {
            anchor,
            layer: Layer::Top,
            size,
            exclusive_zone: -1,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}