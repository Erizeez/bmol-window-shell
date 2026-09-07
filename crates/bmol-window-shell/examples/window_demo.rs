//! Complete authentic macOS Window Demo.
//!
//! Replicates an authentic macOS application window:
//! - Native macOS traffic lights with transparent titlebar and fullsize content view
//! - bmol-window-shell SkyLight compositor blur, continuous corner radius, EDR, and Stage Manager guard
//! - Vector typography rendered with modern antialiased text
//! - Authentic macOS dual-pane layout: frosted sidebar and structured settings cards

use iced::widget::{button, column, container, row, scrollable, slider, space, text, toggler};
use iced::window;
use iced::{Alignment, Color, Element, Length, Padding, Size, Subscription, Task, Theme};

#[derive(Debug, Clone)]
enum Message {
    WindowOpened(window::Id),
    TabSelected(usize),
    ToggleBlur(bool),
    ToggleEdr(bool),
    ToggleGuard(bool),
    CornerRadiusChanged(f64),
    OpacityChanged(f32),
    ResetDefaults,
}

#[derive(Debug)]
struct DemoState {
    window_id: Option<window::Id>,
    active_tab: usize,
    blur_enabled: bool,
    edr_enabled: bool,
    guard_enabled: bool,
    corner_radius: f64,
    opacity: f32,
    native_attached: bool,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            window_id: None,
            active_tab: 0,
            blur_enabled: true,
            edr_enabled: true,
            guard_enabled: true,
            corner_radius: 16.0,
            opacity: 0.88,
            native_attached: false,
        }
    }
}

fn boot() -> (DemoState, Task<Message>) {
    (DemoState::default(), Task::none())
}

fn update(state: &mut DemoState, message: Message) -> Task<Message> {
    match message {
        Message::WindowOpened(id) => {
            state.window_id = Some(id);
            state.native_attached = true;
            let radius = state.corner_radius;
            let edr = state.edr_enabled;

            window::run(id, move |w| {
                if let Ok(handle) = w.window_handle() {
                    let rwh = handle.as_raw();
                    if let Some(target) = bmol_window_shell::desktop_blur_target(rwh) {
                        bmol_window_shell::install_stage_manager_guard(target);
                        bmol_window_shell::configure_window_corner_radius(target, radius);
                        bmol_window_shell::configure_extended_dynamic_range(target, edr);
                        bmol_window_shell::refresh_desktop_blur(target);
                    }
                }
            })
            .discard()
        }
        Message::TabSelected(tab) => {
            state.active_tab = tab;
            Task::none()
        }
        Message::ToggleBlur(enabled) => {
            state.blur_enabled = enabled;
            if let Some(id) = state.window_id {
                window::run(id, move |w| {
                    if let Ok(handle) = w.window_handle() {
                        let rwh = handle.as_raw();
                        if let Some(target) = bmol_window_shell::desktop_blur_target(rwh) {
                            if enabled {
                                bmol_window_shell::refresh_desktop_blur(target);
                            }
                        }
                    }
                })
                .discard()
            } else {
                Task::none()
            }
        }
        Message::ToggleEdr(enabled) => {
            state.edr_enabled = enabled;
            if let Some(id) = state.window_id {
                window::run(id, move |w| {
                    if let Ok(handle) = w.window_handle() {
                        let rwh = handle.as_raw();
                        if let Some(target) = bmol_window_shell::desktop_blur_target(rwh) {
                            bmol_window_shell::configure_extended_dynamic_range(target, enabled);
                        }
                    }
                })
                .discard()
            } else {
                Task::none()
            }
        }
        Message::ToggleGuard(enabled) => {
            state.guard_enabled = enabled;
            if enabled && let Some(id) = state.window_id {
                window::run(id, |w| {
                    if let Ok(handle) = w.window_handle() {
                        let rwh = handle.as_raw();
                        if let Some(target) = bmol_window_shell::desktop_blur_target(rwh) {
                            bmol_window_shell::install_stage_manager_guard(target);
                        }
                    }
                })
                .discard()
            } else {
                Task::none()
            }
        }
        Message::CornerRadiusChanged(radius) => {
            state.corner_radius = radius;
            if let Some(id) = state.window_id {
                window::run(id, move |w| {
                    if let Ok(handle) = w.window_handle() {
                        let rwh = handle.as_raw();
                        if let Some(target) = bmol_window_shell::desktop_blur_target(rwh) {
                            bmol_window_shell::configure_window_corner_radius(target, radius);
                        }
                    }
                })
                .discard()
            } else {
                Task::none()
            }
        }
        Message::OpacityChanged(opacity) => {
            state.opacity = opacity;
            Task::none()
        }
        Message::ResetDefaults => {
            state.blur_enabled = true;
            state.edr_enabled = true;
            state.guard_enabled = true;
            state.corner_radius = 16.0;
            state.opacity = 0.88;
            if let Some(id) = state.window_id {
                window::run(id, move |w| {
                    if let Ok(handle) = w.window_handle() {
                        let rwh = handle.as_raw();
                        if let Some(target) = bmol_window_shell::desktop_blur_target(rwh) {
                            bmol_window_shell::configure_window_corner_radius(target, 16.0);
                            bmol_window_shell::configure_extended_dynamic_range(target, true);
                            bmol_window_shell::refresh_desktop_blur(target);
                        }
                    }
                })
                .discard()
            } else {
                Task::none()
            }
        }
    }
}

fn subscription(_state: &DemoState) -> Subscription<Message> {
    window::open_events().map(Message::WindowOpened)
}

fn view(state: &DemoState) -> Element<'_, Message> {
    let sidebar = view_sidebar(state);
    let content = view_content(state);

    row![sidebar, content].width(Length::Fill).height(Length::Fill).into()
}

fn view_sidebar(state: &DemoState) -> Element<'_, Message> {
    let tabs = [
        ("General", "Window appearance and shell configuration"),
        ("Display & EDR", "Metal layer extended dynamic range"),
        ("Stage Manager", "Compositor blur preservation guard"),
        ("Diagnostics", "Native window handle and surface inspect"),
    ];

    let mut nav_col = column![].spacing(6).width(Length::Fill);

    for (index, (title, _desc)) in tabs.iter().enumerate() {
        let is_selected = state.active_tab == index;

        let item_btn = button(
            row![
                text(if is_selected { "●" } else { "○" }).size(12),
                text(*title).size(13)
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .on_press(Message::TabSelected(index))
        .style(move |_theme, status| {
            if is_selected {
                button::Style {
                    background: Some(Color::from_rgba(0.2, 0.45, 0.9, 0.28).into()),
                    text_color: Color::from_rgb(0.3, 0.7, 1.0),
                    border: iced::Border {
                        color: Color::from_rgba(0.3, 0.6, 1.0, 0.4),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }
            } else {
                let hover = matches!(status, button::Status::Hovered);
                button::Style {
                    background: if hover {
                        Some(Color::from_rgba(1.0, 1.0, 1.0, 0.08).into())
                    } else {
                        None
                    },
                    text_color: if hover {
                        Color::from_rgb(0.9, 0.9, 0.9)
                    } else {
                        Color::from_rgb(0.65, 0.65, 0.68)
                    },
                    border: iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }
        });

        nav_col = nav_col.push(item_btn);
    }

    let status_pill = container(
        column![
            text("bmol-window-shell").size(11).color(Color::from_rgb(0.5, 0.5, 0.55)),
            row![
                text(if state.native_attached { "● Native Attached" } else { "○ Initializing" })
                    .size(11)
                    .color(if state.native_attached {
                        Color::from_rgb(0.2, 0.8, 0.4)
                    } else {
                        Color::from_rgb(0.8, 0.5, 0.2)
                    }),
            ],
        ]
        .spacing(4),
    )
    .padding([8, 12])
    .style(|_theme| container::Style {
        background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.25).into()),
        border: iced::Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    });

    let sidebar_body = column![
        // Space reserved for native macOS Traffic Lights (top left)
        column![].height(Length::Fixed(48.0)),
        text("SETTINGS").size(11).color(Color::from_rgb(0.5, 0.5, 0.55)),
        nav_col,
        column![].height(Length::Fill),
        status_pill,
    ]
    .padding([16, 16])
    .spacing(12)
    .width(Length::Fixed(220.0))
    .height(Length::Fill);

    container(sidebar_body)
        .style(|_theme| container::Style {
            background: Some(Color::from_rgba(0.08, 0.09, 0.11, 0.65).into()),
            border: iced::Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn view_content(state: &DemoState) -> Element<'_, Message> {
    let top_bar = row![
        text(match state.active_tab {
            0 => "General Settings",
            1 => "Display & Dynamic Range",
            2 => "Stage Manager Preservation",
            _ => "System Diagnostics",
        })
        .size(18),
        space::horizontal(),
        button(text("Reset").size(12))
            .padding([4, 12])
            .on_press(Message::ResetDefaults)
            .style(|_theme, _status| button::Style {
                background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.1).into()),
                text_color: Color::from_rgb(0.85, 0.85, 0.85),
                border: iced::Border {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }),
    ]
    .align_y(Alignment::Center)
    .padding([14, 24])
    .height(Length::Fixed(52.0));

    let cards = match state.active_tab {
        0 => view_general_cards(state),
        1 => view_display_cards(state),
        2 => view_guard_cards(state),
        _ => view_diagnostic_cards(state),
    };

    let content_area = column![
        top_bar,
        scrollable(
            column![cards]
                .padding(Padding {
                    top: 8.0,
                    right: 24.0,
                    bottom: 24.0,
                    left: 24.0,
                })
                .spacing(16)
        )
        .height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    container(content_area)
        .style(move |_theme| container::Style {
            background: Some(Color::from_rgba(0.12, 0.13, 0.16, state.opacity).into()),
            ..Default::default()
        })
        .into()
}

fn view_general_cards(state: &DemoState) -> Element<'_, Message> {
    let card1 = card(
        "Window Blur & Native Material",
        "Configures the private SkyLight window background blur and background tint opacity.",
        column![
            row![
                column![
                    text("SkyLight Background Blur").size(14),
                    text("Uses macOS CGSSetWindowBackgroundBlurRadius(28) for authentic desktop blur.")
                        .size(12)
                        .color(Color::from_rgb(0.6, 0.6, 0.65)),
                ]
                .width(Length::Fill),
                toggler(state.blur_enabled).on_toggle(Message::ToggleBlur),
            ]
            .align_y(Alignment::Center),
            card_divider(),
            column![
                row![
                    text("Window Tint Opacity").size(14).width(Length::Fill),
                    text(format!("{:.0}%", state.opacity * 100.0)).size(13),
                ],
                slider(0.3..=1.0, state.opacity, Message::OpacityChanged).step(0.01),
            ]
            .spacing(8),
        ]
        .spacing(12),
    );

    let card2 = card(
        "Window Geometry & Continuous Curvature",
        "Hardware-level continuous squircle corner clipping mask applied via CALayer.",
        column![
            row![
                text("Continuous Corner Radius").size(14).width(Length::Fill),
                text(format!("{:.0} pt", state.corner_radius)).size(13),
            ],
            slider(0.0..=32.0, state.corner_radius, Message::CornerRadiusChanged).step(1.0),
            text("macOS standard window corner radius is typically 10 to 18 points with G2 continuity.")
                .size(12)
                .color(Color::from_rgb(0.6, 0.6, 0.65)),
        ]
        .spacing(8),
    );

    column![card1, card2].spacing(16).into()
}

fn view_display_cards(state: &DemoState) -> Element<'_, Message> {
    let card = card(
        "Extended Dynamic Range (EDR)",
        "Configures CAMetalLayer for floating point linear sRGB color space to preserve specular highlights.",
        column![
            row![
                column![
                    text("Enable Linear EDR Mode").size(14),
                    text("Sets wantsExtendedDynamicRangeContent and kCGColorSpaceExtendedLinearSRGB on CAMetalLayer.")
                        .size(12)
                        .color(Color::from_rgb(0.6, 0.6, 0.65)),
                ]
                .width(Length::Fill),
                toggler(state.edr_enabled).on_toggle(Message::ToggleEdr),
            ]
            .align_y(Alignment::Center),
            card_divider(),
            row![
                text("Supported Target Layer:").size(13).color(Color::from_rgb(0.7, 0.7, 0.75)),
                space::horizontal(),
                text("CAMetalLayer (Liquid Glass / Iced WGPU)").size(13).color(Color::from_rgb(0.3, 0.7, 1.0)),
            ],
        ]
        .spacing(12),
    );

    column![card].spacing(16).into()
}

fn view_guard_cards(state: &DemoState) -> Element<'_, Message> {
    let card = card(
        "Stage Manager Blur Preservation",
        "Prevents translucent window flash when macOS Stage Manager or Mission Control rebuilds the compositor state.",
        column![
            row![
                column![
                    text("Install Re-blur Observer Guard").size(14),
                    text("Monitors NSWorkspaceActiveSpaceDidChange and NSApplicationDidBecomeActive to restore blur.")
                        .size(12)
                        .color(Color::from_rgb(0.6, 0.6, 0.65)),
                ]
                .width(Length::Fill),
                toggler(state.guard_enabled).on_toggle(Message::ToggleGuard),
            ]
            .align_y(Alignment::Center),
            card_divider(),
            text("When switching spaces or switching between app sets in Stage Manager, macOS resets private CGS blur attributes asynchronously. The guard ensures your window retains its frosted glass without flashing unblurred wallpaper.")
                .size(12)
                .color(Color::from_rgb(0.6, 0.6, 0.65)),
        ]
        .spacing(12),
    );

    column![card].spacing(16).into()
}

fn view_diagnostic_cards(state: &DemoState) -> Element<'_, Message> {
    let rows = [
        ("Platform Target", "Apple macOS (AppKit + Quartz / SkyLight)"),
        ("Window Server Connection", "CGSMainConnectionID (Dynamic Symbol)"),
        ("Blur Implementation", "CGSSetWindowBackgroundBlurRadius (28pt)"),
        ("Titlebar Integration", "Fullsize Content View + Transparent Titlebar"),
        ("Traffic Lights", "Native NSWindowCloseButton / Miniaturize / Zoom"),
        (
            "Native Hook Status",
            if state.native_attached {
                "Active (DesktopBlurTarget attached)"
            } else {
                "Waiting for window handle"
            },
        ),
    ];

    let mut col = column![].spacing(10);
    for (idx, (label, val)) in rows.iter().enumerate() {
        if idx > 0 {
            col = col.push(card_divider());
        }
        col = col.push(
            row![
                text(*label).size(13).color(Color::from_rgb(0.65, 0.65, 0.7)),
                space::horizontal(),
                text(*val).size(13).color(Color::from_rgb(0.9, 0.9, 0.9)),
            ]
            .align_y(Alignment::Center),
        );
    }

    let card = card(
        "Shell Runtime Architecture",
        "Live inspection of current bmol-window-shell runtime environment and native bindings.",
        col,
    );

    column![card].spacing(16).into()
}

fn card<'a>(
    title: &'static str,
    subtitle: &'static str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let header = column![
        text(title).size(15),
        text(subtitle).size(12).color(Color::from_rgb(0.6, 0.6, 0.65)),
    ]
    .spacing(4);

    let body = column![header, card_divider(), content.into()].spacing(12);

    container(body)
        .padding(16)
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Color::from_rgba(0.18, 0.20, 0.24, 0.6).into()),
            border: iced::Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn card_divider<'a>() -> Element<'a, Message> {
    container(column![])
        .height(Length::Fixed(1.0))
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.06).into()),
            ..Default::default()
        })
        .into()
}

fn theme(_state: &DemoState) -> Theme {
    Theme::Dark
}

fn style(_state: &DemoState, _theme: &Theme) -> iced::theme::Style {
    iced::theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: Color::from_rgb(0.92, 0.92, 0.92),
    }
}

fn main() -> iced::Result {
    iced::application(boot, update, view)
        .subscription(subscription)
        .theme(theme)
        .style(style)
        .window(window::Settings {
            size: Size::new(960.0, 620.0),
            min_size: Some(Size::new(760.0, 480.0)),
            transparent: true,
            blur: true,
            decorations: true,
            platform_specific: window::settings::PlatformSpecific {
                title_hidden: true,
                titlebar_transparent: true,
                fullsize_content_view: true,
            },
            ..Default::default()
        })
        .run()
}
