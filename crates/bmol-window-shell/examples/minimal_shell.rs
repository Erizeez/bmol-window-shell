//! Minimal Shell Example: 100% Pure Iced + bmol-window-shell.
//!
//! Demonstrates the recommended, production-grade integration pattern for
//! building seamless frameless macOS & Linux desktop applications:
//! - Physical rim isolation (1px light mode, 2px compound dark mode, collapsed in fullscreen)
//! - Loyal drag bar (drag to move, double-click to maximize)
//! - 8-direction interactive border resizer (hardware cursor auto-switch + drag resize)
//! - One-shot native window hardening (EDR, Stage Manager guard, squircle corner masking)

use bmol_window_shell::{
    WindowChromeConfig, WindowShellController, is_system_dark_mode,
};
use iced::widget::{button, column, container, row, space, text};
use iced::window;
use iced::{Alignment, Color, Element, Length, Size, Subscription, Task, Theme};

pub fn main() -> iced::Result {
    iced::application(State::new, State::update, State::view)
        .title("Minimal Shell")
        .window(window::Settings {
            size: Size::new(820.0, 520.0),
            decorations: false,
            transparent: true,
            ..Default::default()
        })
        .subscription(State::subscription)
        .theme(State::theme)
        .run()
}

struct State {
    controller: WindowShellController,
}

#[derive(Debug, Clone)]
enum Message {
    WindowOpened(window::Id),
    WindowResized(Size),
    DragWindow,
    ToggleMaximize,
    ResizeWindow(window::Direction),
    ToggleTheme,
}

impl State {
    fn new() -> (Self, Task<Message>) {
        let is_dark = is_system_dark_mode();
        let config = WindowChromeConfig::separate(38.0);
        let controller = WindowShellController::new(config, is_dark);

        (
            Self { controller },
            Task::none(),
        )
    }

    fn theme(&self) -> Theme {
        if self.controller.is_dark {
            Theme::Dark
        } else {
            Theme::Light
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            window::open_events().map(Message::WindowOpened),
            window::resize_events().map(|(_id, size)| Message::WindowResized(size)),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowOpened(id) => {
                self.controller.set_window_id(id);
                window::run(id, |w| {
                    if let Ok(handle) = w.window_handle() {
                        let _ = bmol_window_shell::setup_native_window(
                            handle.as_raw(),
                            bmol_window_shell::NativeWindowOptions::new()
                                .with_corner_radius(10.0),
                        );
                    }
                })
                .discard()
            }
            Message::WindowResized(size) => {
                self.controller.handle_resized(size.width, size.height);
                Task::none()
            }
            Message::DragWindow => {
                if let Some(id) = self.controller.window_id {
                    window::drag(id)
                } else {
                    Task::none()
                }
            }
            Message::ToggleMaximize => {
                if let Some(id) = self.controller.window_id {
                    window::toggle_maximize(id)
                } else {
                    Task::none()
                }
            }
            Message::ResizeWindow(direction) => {
                if let Some(id) = self.controller.window_id {
                    window::drag_resize(id, direction)
                } else {
                    Task::none()
                }
            }
            Message::ToggleTheme => {
                self.controller.set_dark_mode(!self.controller.is_dark);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let is_dark = self.controller.is_dark;

        // 1. Header Toolbar with title, flexible drag area, and theme toggle
        let header_content = row![
            space().width(Length::Fixed(16.0)),
            text("BMOL Window Shell").size(13).color(if is_dark {
                Color::from_rgb(0.9, 0.9, 0.9)
            } else {
                Color::from_rgb(0.15, 0.15, 0.15)
            }),
            space().width(Length::Fill),
            button(text(if is_dark { "Light Mode" } else { "Dark Mode" }).size(11))
                .padding([3, 8])
                .on_press(Message::ToggleTheme),
            space().width(Length::Fixed(14.0)),
        ]
        .align_y(Alignment::Center)
        .height(Length::Fixed(38.0))
        .width(Length::Fill);

        let draggable_header = self.controller.loyal_drag_bar(
            38.0,
            header_content,
            Message::DragWindow,
            Some(Message::ToggleMaximize),
        );

        // 2. Application Body
        let body_content = container(
            column![
                text("Seamless Frameless Architecture").size(22),
                text("Clean, robust, and zero-compromise platform window shell.").size(14),
                text(format!(
                    "Window Size: {:.0} x {:.0} pt | Safe Area: {:.0} x {:.0} pt",
                    self.controller.window_size.0,
                    self.controller.window_size.1,
                    self.controller.metrics.safe_client_rect.width,
                    self.controller.metrics.safe_client_rect.height,
                ))
                .size(12)
                .color(Color::from_rgb(0.5, 0.5, 0.5)),
            ]
            .spacing(12)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

        let page = column![draggable_header, body_content]
            .width(Length::Fill)
            .height(Length::Fill);

        let page_styled = container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme| container::Style {
                background: Some(if is_dark {
                    Color::from_rgb8(24, 25, 28).into()
                } else {
                    Color::from_rgb8(248, 249, 251).into()
                }),
                border: iced::Border {
                    radius: 10.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        // 3. Wrap with physical non-client rim and interactive 8-direction border resize handles
        self.controller
            .wrap_window_with_resizer(page_styled, 10.0, Message::ResizeWindow)
    }
}
