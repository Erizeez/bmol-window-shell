//! Complete authentic macOS Window Demo with Custom Traffic Lights and Frameless Chrome.
//!
//! Features:
//! - Pure frameless window (`decorations: false`), completely eliminating system black borders and outlines
//! - Hand-crafted Apple-style traffic lights (Close, Minimize, Maximize) with authentic colors, borders, and actions
//! - Two layout strategies:
//!   1. Standalone Titlebar (Separate): 32px white titlebar, left-aligned title strictly 16px from traffic lights
//!   2. Unified Chrome (Integrated): Seamless sidebar extending to top with automatic traffic lights clearance
//! - Full window dragging support across titlebar drag regions
//! - SkyLight real-time blur, continuous squircle curvature, and Stage Manager re-blur protection

use bmol_window_shell::{
    ChromeDrawPlan, ChromeLayoutMode, WindowChromeConfig, WindowChromeMetrics,
};
use iced::widget::{
    button, column, container, row, scrollable, slider, space, text, toggler,
};
use iced::window;
use iced::{Alignment, Color, Element, Length, Padding, Size, Subscription, Task, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutSelection {
    Separate,
    Unified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone)]
enum Message {
    WindowOpened(window::Id),
    WindowResized(Size),
    TabSelected(usize),
    SelectLayout(LayoutSelection),
    SelectTheme(ThemePreference),
    SystemThemeChanged(iced::theme::Mode),
    PollSystemTheme,
    SeparateHeightChanged(f32),
    UnifiedHeightChanged(f32),
    ToggleHitboxes(bool),
    ToggleBlur(bool),
    ToggleShadow(bool),
    ToggleEdr(bool),
    ToggleGuard(bool),
    CornerRadiusChanged(f64),
    OpacityChanged(f32),
    ResetDefaults,
    CloseWindow,
    MinimizeWindow,
    ToggleMaximize,
    DragWindow,
}

#[derive(Debug)]
struct DemoState {
    window_id: Option<window::Id>,
    window_size: Size,
    active_tab: usize,
    layout_selection: LayoutSelection,
    theme_preference: ThemePreference,
    system_theme: iced::theme::Mode,
    separate_titlebar_height: f32,
    unified_header_height: f32,
    sidebar_width: f32,
    show_hitboxes: bool,
    blur_enabled: bool,
    system_shadow: bool,
    edr_enabled: bool,
    guard_enabled: bool,
    corner_radius: f64,
    opacity: f32,
    native_attached: bool,
    metrics: WindowChromeMetrics,
}

impl Default for DemoState {
    fn default() -> Self {
        let default_config = WindowChromeConfig::separate(32.0);
        let metrics = WindowChromeMetrics::compute(980.0, 640.0, &default_config);
        let initial_dark = bmol_window_shell::is_system_dark_mode();
        let initial_theme = if initial_dark {
            iced::theme::Mode::Dark
        } else {
            iced::theme::Mode::Light
        };

        Self {
            window_id: None,
            window_size: Size::new(980.0, 640.0),
            active_tab: 0,
            layout_selection: LayoutSelection::Separate,
            theme_preference: ThemePreference::System,
            system_theme: initial_theme,
            separate_titlebar_height: 32.0,
            unified_header_height: 54.0,
            sidebar_width: 220.0,
            show_hitboxes: false,
            blur_enabled: true,
            system_shadow: false, // Default to FALSE! Eliminates the automatic 1px dark rim macOS draws around windows
            edr_enabled: true,
            guard_enabled: true,
            corner_radius: 16.0,
            opacity: 0.88,
            native_attached: false,
            metrics,
        }
    }
}

impl DemoState {
    fn effective_mode(&self) -> iced::theme::Mode {
        match self.theme_preference {
            ThemePreference::System => match self.system_theme {
                iced::theme::Mode::Light => iced::theme::Mode::Light,
                iced::theme::Mode::Dark => iced::theme::Mode::Dark,
                iced::theme::Mode::None => {
                    if bmol_window_shell::is_system_dark_mode() {
                        iced::theme::Mode::Dark
                    } else {
                        iced::theme::Mode::Light
                    }
                }
            },
            ThemePreference::Light => iced::theme::Mode::Light,
            ThemePreference::Dark => iced::theme::Mode::Dark,
        }
    }

    fn is_dark(&self) -> bool {
        self.effective_mode() != iced::theme::Mode::Light
    }

    fn native_appearance(&self) -> bmol_window_shell::WindowAppearance {
        match self.theme_preference {
            ThemePreference::System => bmol_window_shell::WindowAppearance::System,
            ThemePreference::Light => bmol_window_shell::WindowAppearance::Light,
            ThemePreference::Dark => bmol_window_shell::WindowAppearance::Dark,
        }
    }

    fn update_metrics(&mut self) {
        let config = match self.layout_selection {
            LayoutSelection::Separate => {
                WindowChromeConfig::separate(self.separate_titlebar_height)
            }
            LayoutSelection::Unified => {
                WindowChromeConfig::unified(self.unified_header_height, Some(self.sidebar_width))
            }
        };
        self.metrics = WindowChromeMetrics::compute(
            self.window_size.width,
            self.window_size.height,
            &config,
        );
    }
}

fn boot() -> (DemoState, Task<Message>) {
    (
        DemoState::default(),
        iced::system::theme().map(Message::SystemThemeChanged),
    )
}

fn update(state: &mut DemoState, message: Message) -> Task<Message> {
    let task = match message {
        Message::WindowOpened(id) => {
            state.window_id = Some(id);
            state.native_attached = true;
            let radius = state.corner_radius;
            let edr = state.edr_enabled;
            let shadow = state.system_shadow;
            let appearance = state.native_appearance();

            window::run(id, move |w| {
                if let Ok(handle) = w.window_handle() {
                    let rwh = handle.as_raw();
                    if let Some(target) = bmol_window_shell::desktop_blur_target(rwh) {
                        bmol_window_shell::install_stage_manager_guard(target);
                        bmol_window_shell::configure_window_corner_radius(target, radius);
                        bmol_window_shell::configure_window_shadow(target, shadow);
                        bmol_window_shell::configure_window_appearance(target, appearance);
                        bmol_window_shell::configure_extended_dynamic_range(target, edr);
                        bmol_window_shell::refresh_desktop_blur(target);
                    }
                }
            })
            .discard()
        }
        Message::WindowResized(size) => {
            state.window_size = size;
            Task::none()
        }
        Message::TabSelected(tab) => {
            state.active_tab = tab;
            Task::none()
        }
        Message::SelectLayout(layout) => {
            state.layout_selection = layout;
            Task::none()
        }
        Message::SelectTheme(pref) => {
            state.theme_preference = pref;
            let appearance = state.native_appearance();
            if let Some(id) = state.window_id {
                window::run(id, move |w| {
                    if let Ok(handle) = w.window_handle() {
                        let rwh = handle.as_raw();
                        if let Some(target) = bmol_window_shell::desktop_blur_target(rwh) {
                            bmol_window_shell::configure_window_appearance(target, appearance);
                        }
                    }
                })
                .discard()
            } else {
                Task::none()
            }
        }
        Message::SystemThemeChanged(mode) => {
            state.system_theme = mode;
            if state.theme_preference == ThemePreference::System {
                if let Some(id) = state.window_id {
                    let appearance = state.native_appearance();
                    window::run(id, move |w| {
                        if let Ok(handle) = w.window_handle() {
                            let rwh = handle.as_raw();
                            if let Some(target) = bmol_window_shell::desktop_blur_target(rwh) {
                                bmol_window_shell::configure_window_appearance(target, appearance);
                            }
                        }
                    })
                    .discard()
                } else {
                    Task::none()
                }
            } else {
                Task::none()
            }
        }
        Message::PollSystemTheme => {
            let current_dark = bmol_window_shell::is_system_dark_mode();
            let current_mode = if current_dark {
                iced::theme::Mode::Dark
            } else {
                iced::theme::Mode::Light
            };
            if state.system_theme != current_mode {
                state.system_theme = current_mode;
                if state.theme_preference == ThemePreference::System {
                    if let Some(id) = state.window_id {
                        let appearance = state.native_appearance();
                        window::run(id, move |w| {
                            if let Ok(handle) = w.window_handle() {
                                let rwh = handle.as_raw();
                                if let Some(target) = bmol_window_shell::desktop_blur_target(rwh) {
                                    bmol_window_shell::configure_window_appearance(target, appearance);
                                }
                            }
                        })
                        .discard()
                    } else {
                        Task::none()
                    }
                } else {
                    Task::none()
                }
            } else {
                Task::none()
            }
        }
        Message::SeparateHeightChanged(h) => {
            state.separate_titlebar_height = h;
            Task::none()
        }
        Message::UnifiedHeightChanged(h) => {
            state.unified_header_height = h;
            Task::none()
        }
        Message::ToggleHitboxes(enabled) => {
            state.show_hitboxes = enabled;
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
        Message::ToggleShadow(enabled) => {
            state.system_shadow = enabled;
            if let Some(id) = state.window_id {
                window::run(id, move |w| {
                    if let Ok(handle) = w.window_handle() {
                        let rwh = handle.as_raw();
                        if let Some(target) = bmol_window_shell::desktop_blur_target(rwh) {
                            bmol_window_shell::configure_window_shadow(target, enabled);
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
            state.layout_selection = LayoutSelection::Separate;
            state.separate_titlebar_height = 32.0;
            state.unified_header_height = 54.0;
            state.show_hitboxes = false;
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
        Message::CloseWindow => {
            if let Some(id) = state.window_id {
                window::close(id)
            } else {
                Task::none()
            }
        }
        Message::MinimizeWindow => {
            if let Some(id) = state.window_id {
                window::minimize(id, true)
            } else {
                Task::none()
            }
        }
        Message::ToggleMaximize => {
            if let Some(id) = state.window_id {
                window::toggle_maximize(id)
            } else {
                Task::none()
            }
        }
        Message::DragWindow => {
            if let Some(id) = state.window_id {
                window::drag(id)
            } else {
                Task::none()
            }
        }
    };

    state.update_metrics();
    task
}

fn subscription(_state: &DemoState) -> Subscription<Message> {
    Subscription::batch([
        window::open_events().map(Message::WindowOpened),
        window::resize_events().map(|(_id, size)| Message::WindowResized(size)),
        iced::system::theme_changes().map(Message::SystemThemeChanged),
        iced::time::every(std::time::Duration::from_millis(250)).map(|_| Message::PollSystemTheme),
    ])
}

fn view(state: &DemoState) -> Element<'_, Message> {
    let plan = ChromeDrawPlan::from_metrics(&state.metrics);

    let content = match state.layout_selection {
        LayoutSelection::Separate => view_separate_window(state, plan),
        LayoutSelection::Unified => view_unified_window(state, plan),
    };

    // Wrap the entire window in a continuous rounded container with transparent edges
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme| container::Style {
            border: iced::Border {
                radius: 16.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

// =========================================================================
// Hand-crafted Apple-style Traffic Lights (Close / Minimize / Zoom)
// =========================================================================

fn view_traffic_lights() -> Element<'static, Message> {
    let make_circle = |fill: Color, stroke: Color, msg: Message| {
        button(space::horizontal().width(Length::Fixed(12.0)).height(Length::Fixed(12.0)))
            .padding(0)
            .width(Length::Fixed(12.0))
            .height(Length::Fixed(12.0))
            .on_press(msg)
            .style(move |_theme, status| {
                let is_hover = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: Some(fill.into()),
                    border: iced::Border {
                        color: if is_hover {
                            stroke
                        } else {
                            Color { a: 0.35, ..stroke }
                        },
                        width: 0.5,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }
            })
    };

    // Authentic Apple palette
    let close_btn = make_circle(
        Color::from_rgb8(0xFF, 0x5F, 0x56),
        Color::from_rgb8(0xE0, 0x44, 0x3E),
        Message::CloseWindow,
    );

    let min_btn = make_circle(
        Color::from_rgb8(0xFF, 0xBD, 0x2E),
        Color::from_rgb8(0xDE, 0xA1, 0x23),
        Message::MinimizeWindow,
    );

    let zoom_btn = make_circle(
        Color::from_rgb8(0x27, 0xC9, 0x3F),
        Color::from_rgb8(0x1A, 0xAB, 0x29),
        Message::ToggleMaximize,
    );

    row![close_btn, min_btn, zoom_btn]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
}

fn view_theme_toggle(state: &DemoState) -> Element<'_, Message> {
    let make_item = |label: &'static str, pref: ThemePreference| {
        let is_active = state.theme_preference == pref;
        let is_dark = state.is_dark();
        button(text(label).size(11))
            .padding([3, 7])
            .on_press(Message::SelectTheme(pref))
            .style(move |_theme, status| {
                if is_active {
                    button::Style {
                        background: Some(Color::from_rgba(0.0, 0.48, 1.0, 0.85).into()),
                        text_color: Color::WHITE,
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                } else {
                    let is_hover = matches!(status, button::Status::Hovered);
                    button::Style {
                        background: if is_hover {
                            Some(if is_dark {
                                Color::from_rgba(1.0, 1.0, 1.0, 0.08).into()
                            } else {
                                Color::from_rgba(0.0, 0.0, 0.0, 0.06).into()
                            })
                        } else {
                            None
                        },
                        text_color: if is_dark {
                            Color::from_rgb(0.65, 0.65, 0.70)
                        } else {
                            Color::from_rgb(0.35, 0.36, 0.40)
                        },
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }
            })
    };

    let container_bg = if state.is_dark() {
        Color::from_rgba(1.0, 1.0, 1.0, 0.06)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.05)
    };

    container(
        row![
            make_item("Auto", ThemePreference::System),
            make_item("Light", ThemePreference::Light),
            make_item("Dark", ThemePreference::Dark),
        ]
        .spacing(2)
        .align_y(Alignment::Center),
    )
    .padding(2)
    .style(move |_theme| container::Style {
        background: Some(container_bg.into()),
        border: iced::Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

// =========================================================================
// Layout 1: Standalone Titlebar (Separate) - 32px White Bar, 16px Clearance
// =========================================================================

fn view_separate_window(state: &DemoState, _plan: ChromeDrawPlan) -> Element<'_, Message> {
    let titlebar_height = state.metrics.header_rect.height;
    let is_dark = state.is_dark();

    let titlebar_bg = if is_dark {
        Color::from_rgba(0.14, 0.15, 0.18, 0.98)
    } else {
        Color::from_rgba(1.0, 1.0, 1.0, 0.98)
    };
    let title_color = if is_dark {
        Color::from_rgb(0.92, 0.92, 0.95)
    } else {
        Color::from_rgb(0.12, 0.13, 0.15)
    };
    let sidebar_bg = if is_dark {
        Color::from_rgba(0.09, 0.10, 0.12, 0.5)
    } else {
        Color::from_rgba(0.94, 0.94, 0.96, 0.7)
    };
    let workspace_bg = if is_dark {
        Color::from_rgba(0.12, 0.13, 0.16, state.opacity)
    } else {
        Color::from_rgba(0.98, 0.98, 1.0, state.opacity)
    };

    let mode_switch = row![
        view_theme_toggle(state),
        column![].width(Length::Fixed(6.0)),
        button(text("Separate (Active)").size(11))
            .padding([3, 8])
            .style(|_theme, _status| button::Style {
                background: Some(Color::from_rgba(0.0, 0.48, 1.0, 0.12).into()),
                text_color: Color::from_rgb(0.0, 0.42, 0.90),
                border: iced::Border {
                    radius: 4.0.into(),
                    color: Color::from_rgba(0.0, 0.48, 1.0, 0.30),
                    width: 1.0,
                },
                ..Default::default()
            }),
        button(text("Switch to Unified").size(11))
            .padding([3, 8])
            .on_press(Message::SelectLayout(LayoutSelection::Unified))
            .style(move |_theme, status| button::Style {
                background: if matches!(status, button::Status::Hovered) {
                    Some(if is_dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.08).into()
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.05).into()
                    })
                } else {
                    None
                },
                text_color: if is_dark {
                    Color::from_rgb(0.7, 0.7, 0.75)
                } else {
                    Color::from_rgb(0.35, 0.36, 0.40)
                },
                border: iced::Border {
                    radius: 4.0.into(),
                    color: if is_dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.10)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.10)
                    },
                    width: 1.0,
                },
                ..Default::default()
            }),
        button(text("Reset").size(11))
            .padding([3, 8])
            .on_press(Message::ResetDefaults)
            .style(move |_theme, status| button::Style {
                background: if matches!(status, button::Status::Hovered) {
                    Some(if is_dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.08).into()
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.05).into()
                    })
                } else {
                    None
                },
                text_color: if is_dark {
                    Color::from_rgb(0.7, 0.7, 0.75)
                } else {
                    Color::from_rgb(0.35, 0.36, 0.40)
                },
                border: iced::Border {
                    radius: 4.0.into(),
                    color: if is_dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.10)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.10)
                    },
                    width: 1.0,
                },
                ..Default::default()
            }),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let titlebar_content = row![
        // 1. Left leading edge margin (16px)
        column![].width(Length::Fixed(16.0)),
        // 2. Custom Apple traffic lights
        view_traffic_lights(),
        // 3. Exactly 16px clearance between traffic lights and title
        column![].width(Length::Fixed(16.0)),
        // 4. Left-aligned title text (adaptive sharp text on titlebar)
        text("BMOL Window Shell")
            .size(13)
            .color(title_color),
        // 5. Draggable titlebar area in the center: drag window anywhere here
        button(space::horizontal().height(Length::Fill))
            .padding(0)
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::DragWindow)
            .style(|_theme, _status| button::Style {
                background: None,
                ..Default::default()
            }),
        // 6. Right action area
        mode_switch,
        column![].width(Length::Fixed(16.0)),
    ]
    .align_y(Alignment::Center)
    .height(Length::Fixed(titlebar_height))
    .width(Length::Fill);

    let titlebar_container = container(titlebar_content)
        .style(move |_theme| container::Style {
            background: Some(titlebar_bg.into()),
            border: iced::Border {
                radius: iced::border::Radius {
                    top_left: 16.0,
                    top_right: 16.0,
                    bottom_right: 0.0,
                    bottom_left: 0.0,
                },
                ..Default::default()
            },
            ..Default::default()
        });

    let separator = container(column![]).height(Length::Fixed(0.0));

    // 2. Content below titlebar (safe area)
    let sidebar = view_sidebar_items(state);
    let main_cards = view_content_cards(state);

    let content_split = row![
        container(sidebar)
            .width(Length::Fixed(state.sidebar_width))
            .height(Length::Fill)
            .style(move |_theme| container::Style {
                background: Some(sidebar_bg.into()),
                border: iced::Border {
                    radius: iced::border::Radius {
                        top_left: 0.0,
                        top_right: 0.0,
                        bottom_right: 0.0,
                        bottom_left: 16.0,
                    },
                    ..Default::default()
                },
                ..Default::default()
            }),
        container(main_cards)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme| container::Style {
                background: Some(workspace_bg.into()),
                border: iced::Border {
                    radius: iced::border::Radius {
                        top_left: 0.0,
                        top_right: 0.0,
                        bottom_right: 16.0,
                        bottom_left: 0.0,
                    },
                    ..Default::default()
                },
                ..Default::default()
            }),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    let mut outer_col = column![titlebar_container, separator, content_split]
        .width(Length::Fill)
        .height(Length::Fill);

    if state.show_hitboxes {
        outer_col = outer_col.push(view_hitboxes_overlay_banner(&state.metrics));
    }

    outer_col.into()
}

// =========================================================================
// Layout 2: Unified (Integrated) Chrome with Custom Traffic Lights
// =========================================================================

fn view_unified_window(state: &DemoState, _plan: ChromeDrawPlan) -> Element<'_, Message> {
    let sidebar_inner = column![
        // Top row with custom traffic lights
        row![
            column![].width(Length::Fixed(4.0)),
            view_traffic_lights(),
        ]
        .height(Length::Fixed(34.0))
        .align_y(Alignment::Center),
        text("NAVIGATION")
            .size(11)
            .color(Color::from_rgb(0.5, 0.5, 0.55)),
        view_sidebar_items(state),
    ]
    .padding(Padding {
        top: 8.0,
        right: 16.0,
        bottom: 16.0,
        left: 16.0,
    })
    .spacing(12)
    .height(Length::Fill);

    let is_dark = state.is_dark();
    let unified_sidebar_bg = if is_dark {
        Color::from_rgba(0.08, 0.09, 0.11, 0.65)
    } else {
        Color::from_rgba(0.92, 0.93, 0.95, 0.85)
    };
    let unified_workspace_bg = if is_dark {
        Color::from_rgba(0.12, 0.13, 0.16, state.opacity)
    } else {
        Color::from_rgba(0.98, 0.98, 1.0, state.opacity)
    };

    let sidebar_container = container(sidebar_inner)
        .width(Length::Fixed(state.sidebar_width))
        .height(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(unified_sidebar_bg.into()),
            border: iced::Border {
                radius: iced::border::Radius {
                    top_left: 16.0,
                    top_right: 0.0,
                    bottom_right: 0.0,
                    bottom_left: 16.0,
                },
                ..Default::default()
            },
            ..Default::default()
        });

    let v_separator = container(column![]).width(Length::Fixed(0.0));

    // 2. Right Content Area: Toolbar on top + cards
    let top_toolbar = row![
        text(match state.active_tab {
            0 => "Window Architecture",
            1 => "Layout & Collision Hitboxes",
            2 => "Display & EDR Dynamic Range",
            _ => "Compositor & Stage Manager",
        })
        .size(16),
        // Draggable empty space in the toolbar
        button(space::horizontal().height(Length::Fill))
            .padding(0)
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::DragWindow)
            .style(|_theme, _status| button::Style {
                background: None,
                ..Default::default()
            }),
        row![
            view_theme_toggle(state),
            column![].width(Length::Fixed(6.0)),
            button(text("Switch to Separate").size(11))
                .padding([4, 8])
                .on_press(Message::SelectLayout(LayoutSelection::Separate))
                .style(move |_theme, status| button::Style {
                    background: if matches!(status, button::Status::Hovered) {
                        Some(if is_dark {
                            Color::from_rgba(1.0, 1.0, 1.0, 0.1).into()
                        } else {
                            Color::from_rgba(0.0, 0.0, 0.0, 0.05).into()
                        })
                    } else {
                        None
                    },
                    text_color: if is_dark {
                        Color::from_rgb(0.7, 0.7, 0.75)
                    } else {
                        Color::from_rgb(0.35, 0.36, 0.40)
                    },
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            button(text("Unified (Active)").size(11))
                .padding([4, 8])
                .style(|_theme, _status| button::Style {
                    background: Some(Color::from_rgba(0.2, 0.5, 1.0, 0.4).into()),
                    text_color: Color::from_rgb(0.4, 0.8, 1.0),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    ]
    .align_y(Alignment::Center)
    .padding([12, 24])
    .height(Length::Fixed(state.metrics.header_rect.height));

    let main_body = view_content_cards(state);

    let content_area = column![top_toolbar, main_body]
        .width(Length::Fill)
        .height(Length::Fill);

    let content_container = container(content_area)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(unified_workspace_bg.into()),
            border: iced::Border {
                radius: iced::border::Radius {
                    top_left: 0.0,
                    top_right: 16.0,
                    bottom_right: 16.0,
                    bottom_left: 0.0,
                },
                ..Default::default()
            },
            ..Default::default()
        });

    let outer_row = row![sidebar_container, v_separator, content_container]
        .width(Length::Fill)
        .height(Length::Fill);

    if state.show_hitboxes {
        column![outer_row, view_hitboxes_overlay_banner(&state.metrics)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        outer_row.into()
    }
}

// =========================================================================
// Shared Components: Sidebar Items & Content Cards
// =========================================================================

fn view_sidebar_items(state: &DemoState) -> Element<'_, Message> {
    let tabs = [
        ("Architecture", "Window Shell overview"),
        ("Hitboxes & Layout", "Collision & insets clearance"),
        ("Display & EDR", "Extended linear range"),
        ("Stage Manager", "Compositor preservation"),
    ];

    let mut nav_col = column![].spacing(6).width(Length::Fill);

    for (index, (title, _desc)) in tabs.iter().enumerate() {
        let is_selected = state.active_tab == index;

        let item_btn = button(
            row![
                text(if is_selected { "●" } else { "○" }).size(11),
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
                let is_dark = state.is_dark();
                button::Style {
                    background: if hover {
                        Some(if is_dark {
                            Color::from_rgba(1.0, 1.0, 1.0, 0.08).into()
                        } else {
                            Color::from_rgba(0.0, 0.0, 0.0, 0.06).into()
                        })
                    } else {
                        None
                    },
                    text_color: if hover {
                        if is_dark {
                            Color::from_rgb(0.95, 0.95, 0.98)
                        } else {
                            Color::from_rgb(0.10, 0.10, 0.15)
                        }
                    } else {
                        if is_dark {
                            Color::from_rgb(0.65, 0.65, 0.70)
                        } else {
                            Color::from_rgb(0.35, 0.36, 0.40)
                        }
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

    let is_dark = state.is_dark();
    let status_pill = container(
        column![
            text("bmol-window-shell").size(11).color(if is_dark {
                Color::from_rgb(0.5, 0.5, 0.55)
            } else {
                Color::from_rgb(0.4, 0.4, 0.45)
            }),
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

    column![nav_col, space::horizontal(), status_pill]
        .spacing(12)
        .height(Length::Fill)
        .into()
}

fn view_content_cards(state: &DemoState) -> Element<'_, Message> {
    let cards = match state.active_tab {
        0 => view_tab_architecture(state),
        1 => view_tab_hitboxes_and_layout(state),
        2 => view_tab_edr(state),
        _ => view_tab_stage_manager(state),
    };

    scrollable(
        column![cards]
            .padding(Padding {
                top: 8.0,
                right: 24.0,
                bottom: 24.0,
                left: 24.0,
            })
            .spacing(16),
    )
    .height(Length::Fill)
    .into()
}

fn view_tab_architecture(state: &DemoState) -> Element<'_, Message> {
    let card_layout = card(
        "Window Chrome Layout Modes",
        "Choose between a standalone top titlebar or an integrated unified chrome.",
        column![
            row![
                button(text("Separate (Standalone Titlebar)").size(13))
                    .padding([8, 16])
                    .on_press(Message::SelectLayout(LayoutSelection::Separate))
                    .style(|_theme, _status| {
                        button::Style {
                            background: Some(Color::from_rgba(0.2, 0.4, 0.8, 0.3).into()),
                            text_color: Color::from_rgb(0.85, 0.85, 0.9),
                            border: iced::Border {
                                radius: 6.0.into(),
                                color: Color::from_rgba(0.3, 0.6, 1.0, 0.4),
                                width: 1.0,
                            },
                            ..Default::default()
                        }
                    }),
                button(text("Unified (Integrated Chrome)").size(13))
                    .padding([8, 16])
                    .on_press(Message::SelectLayout(LayoutSelection::Unified))
                    .style(|_theme, _status| button::Style {
                        background: Some(Color::from_rgba(0.2, 0.4, 0.8, 0.3).into()),
                        text_color: Color::from_rgb(0.85, 0.85, 0.9),
                        border: iced::Border {
                            radius: 6.0.into(),
                            color: Color::from_rgba(0.3, 0.6, 1.0, 0.4),
                            width: 1.0,
                        },
                        ..Default::default()
                    }),
            ]
            .spacing(12),
            card_divider(),
            text(match state.layout_selection {
                LayoutSelection::Separate => {
                    "Current: Separate Mode. 32px pure white titlebar with custom traffic lights. Content strictly starts below the titlebar."
                }
                LayoutSelection::Unified => {
                    "Current: Unified Mode. Content & sidebar extend to top edges. Downstream applications avoid traffic lights using computed collision hitboxes."
                }
            })
            .size(12)
            .color(Color::from_rgb(0.7, 0.75, 0.8)),
        ]
        .spacing(10),
    );

    let card_height_config = card(
        "User Specified Header & Titlebar Height",
        "Specify custom chrome dimensions. bmol-window-shell computes collision insets automatically.",
        column![
            column![
                row![
                    text("Separate Titlebar Height").size(13).width(Length::Fill),
                    text(format!("{:.0} pt", state.separate_titlebar_height)).size(13),
                ],
                slider(28.0..=64.0, state.separate_titlebar_height, Message::SeparateHeightChanged)
                    .step(1.0),
            ]
            .spacing(6),
            card_divider(),
            column![
                row![
                    text("Unified Header Clearance Height").size(13).width(Length::Fill),
                    text(format!("{:.0} pt", state.unified_header_height)).size(13),
                ],
                slider(40.0..=80.0, state.unified_header_height, Message::UnifiedHeightChanged)
                    .step(1.0),
            ]
            .spacing(6),
        ]
        .spacing(12),
    );

    let card_blur = card(
        "Compositor & Window Appearance",
        "Control SkyLight blur, corner curvature, and glass opacity.",
        column![
            row![
                column![
                    text("SkyLight Compositor Blur").size(14),
                    text("AppKit private CGSSetWindowBackgroundBlurRadius(28).")
                        .size(12)
                        .color(Color::from_rgb(0.6, 0.6, 0.65)),
                ]
                .width(Length::Fill),
                toggler(state.blur_enabled).on_toggle(Message::ToggleBlur),
            ]
            .align_y(Alignment::Center),
            card_divider(),
            row![
                column![
                    text("macOS WindowServer Shadow & 1px Rim").size(14),
                    text("Turning this OFF completely removes the automatic 1px dark border drawn by macOS.")
                        .size(12)
                        .color(Color::from_rgb(0.6, 0.6, 0.65)),
                ]
                .width(Length::Fill),
                toggler(state.system_shadow).on_toggle(Message::ToggleShadow),
            ]
            .align_y(Alignment::Center),
            card_divider(),
            column![
                row![
                    text("Window Tint Opacity").size(13).width(Length::Fill),
                    text(format!("{:.0}%", state.opacity * 100.0)).size(13),
                ],
                slider(0.3..=1.0, state.opacity, Message::OpacityChanged).step(0.01),
            ]
            .spacing(6),
            card_divider(),
            column![
                row![
                    text("Continuous Corner Radius (Squircle)").size(13).width(Length::Fill),
                    text(format!("{:.0} pt", state.corner_radius)).size(13),
                ],
                slider(0.0..=32.0, state.corner_radius, Message::CornerRadiusChanged).step(1.0),
            ]
            .spacing(6),
        ]
        .spacing(12),
    );

    column![card_layout, card_height_config, card_blur].spacing(16).into()
}

fn view_tab_hitboxes_and_layout(state: &DemoState) -> Element<'_, Message> {
    let metrics = &state.metrics;

    let card_toggle = card(
        "Collision Hitboxes & Insets Inspector",
        "Enable visualization overlay to inspect computed bounds provided to downstream apps.",
        column![
            row![
                column![
                    text("Show Collision Hitboxes Overlay").size(14),
                    text("Renders debug collision boxes for traffic lights, drag zones, and content bounds.")
                        .size(12)
                        .color(Color::from_rgb(0.6, 0.6, 0.65)),
                ]
                .width(Length::Fill),
                toggler(state.show_hitboxes).on_toggle(Message::ToggleHitboxes),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(8),
    );

    let rows = [
        (
            "Window Dimensions",
            format!("{:.0} × {:.0} pt", metrics.window_size.0, metrics.window_size.1),
        ),
        (
            "Active Layout Mode",
            match metrics.mode {
                ChromeLayoutMode::Separate { titlebar_height } => {
                    format!("Separate (Titlebar Height: {:.0} pt)", titlebar_height)
                }
                ChromeLayoutMode::Unified { header_height, .. } => {
                    format!("Unified (Header Height: {:.0} pt)", header_height)
                }
            },
        ),
        (
            "Traffic Lights Hitbox",
            format!(
                "x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}",
                metrics.traffic_lights_hitbox.x,
                metrics.traffic_lights_hitbox.y,
                metrics.traffic_lights_hitbox.width,
                metrics.traffic_lights_hitbox.height
            ),
        ),
        (
            "Traffic Lights Exclusion Zone",
            format!(
                "w: {:.1} pt (Downstream apps MUST avoid interactive widgets here)",
                metrics.traffic_lights_exclusion_zone.width
            ),
        ),
        (
            "Content Safe Bounds",
            format!(
                "x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}",
                metrics.content_rect.x,
                metrics.content_rect.y,
                metrics.content_rect.width,
                metrics.content_rect.height
            ),
        ),
        (
            "Draggable Region Count",
            format!("{} active drag zone(s)", metrics.drag_regions.len()),
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
                text(val.clone()).size(13).color(Color::from_rgb(0.9, 0.9, 0.9)),
            ]
            .align_y(Alignment::Center),
        );
    }

    let card_table = card(
        "Computed Downstream Bounds Table",
        "Exact AABB metrics provided by bmol-window-shell for downstream rendering.",
        col,
    );

    column![card_toggle, card_table].spacing(16).into()
}

fn view_tab_edr(_state: &DemoState) -> Element<'_, Message> {
    let card = card(
        "Extended Dynamic Range (EDR)",
        "CAMetalLayer 16-bit float linear color space configuration.",
        column![
            row![
                column![
                    text("Enable Linear EDR Mode").size(14),
                    text("Sets wantsExtendedDynamicRangeContent and kCGColorSpaceExtendedLinearSRGB.")
                        .size(12)
                        .color(Color::from_rgb(0.6, 0.6, 0.65)),
                ]
                .width(Length::Fill),
                toggler(true).on_toggle(Message::ToggleEdr),
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

fn view_tab_stage_manager(_state: &DemoState) -> Element<'_, Message> {
    let card = card(
        "Stage Manager Blur Preservation",
        "Prevents translucent window flash during macOS Stage Manager and Mission Control transitions.",
        column![
            row![
                column![
                    text("Install Re-blur Observer Guard").size(14),
                    text("Monitors NSWorkspaceActiveSpaceDidChange to restore private CGS blur radius.")
                        .size(12)
                        .color(Color::from_rgb(0.6, 0.6, 0.65)),
                ]
                .width(Length::Fill),
                toggler(true).on_toggle(Message::ToggleGuard),
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

fn view_hitboxes_overlay_banner(metrics: &WindowChromeMetrics) -> Element<'_, Message> {
    let banner = row![
        text("HITBOX OVERLAY ACTIVE:").size(11).color(Color::from_rgb(1.0, 0.8, 0.2)),
        text(format!(
            "Traffic Lights Exclusion: w={:.0}, h={:.0}",
            metrics.traffic_lights_exclusion_zone.width,
            metrics.traffic_lights_exclusion_zone.height
        ))
        .size(11)
        .color(Color::from_rgb(1.0, 0.4, 0.4)),
        text("|").size(11).color(Color::from_rgb(0.4, 0.4, 0.4)),
        text(format!(
            "Content Bounds: y={:.0}, h={:.0}",
            metrics.content_rect.y,
            metrics.content_rect.height
        ))
        .size(11)
        .color(Color::from_rgb(0.3, 0.8, 0.4)),
        space::horizontal(),
        text("Downstream widgets safely cleared").size(11).color(Color::from_rgb(0.6, 0.6, 0.65)),
    ]
    .spacing(10)
    .padding([6, 16])
    .align_y(Alignment::Center);

    container(banner)
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Color::from_rgba(0.08, 0.10, 0.14, 0.95).into()),
            border: iced::Border {
                color: Color::from_rgba(1.0, 0.7, 0.2, 0.4),
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn card<'a>(
    title: &'static str,
    subtitle: &'static str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let body = column![
        column![
            text(title).size(15),
            text(subtitle).size(12).color(Color::from_rgb(0.55, 0.55, 0.60)),
        ]
        .spacing(4),
        card_divider(),
        content.into()
    ]
    .spacing(12);

    container(body)
        .padding(16)
        .width(Length::Fill)
        .style(|theme| {
            let is_light = matches!(
                theme,
                Theme::Light
                    | Theme::CatppuccinLatte
                    | Theme::SolarizedLight
                    | Theme::TokyoNightLight
            );
            container::Style {
                background: Some(if is_light {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.85).into()
                } else {
                    Color::from_rgba(0.18, 0.20, 0.24, 0.6).into()
                }),
                border: iced::Border {
                    color: if is_light {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.06)
                    } else {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.08)
                    },
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn card_divider<'a>() -> Element<'a, Message> {
    container(column![])
        .height(Length::Fixed(1.0))
        .width(Length::Fill)
        .style(|theme| {
            let is_light = matches!(
                theme,
                Theme::Light
                    | Theme::CatppuccinLatte
                    | Theme::SolarizedLight
                    | Theme::TokyoNightLight
            );
            container::Style {
                background: Some(if is_light {
                    Color::from_rgba(0.0, 0.0, 0.0, 0.06).into()
                } else {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.06).into()
                }),
                ..Default::default()
            }
        })
        .into()
}

fn theme(state: &DemoState) -> Theme {
    if state.is_dark() {
        Theme::Dark
    } else {
        Theme::Light
    }
}

fn style(state: &DemoState, _theme: &Theme) -> iced::theme::Style {
    iced::theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: if state.is_dark() {
            Color::from_rgb(0.92, 0.92, 0.92)
        } else {
            Color::from_rgb(0.12, 0.13, 0.15)
        },
    }
}

fn main() -> iced::Result {
    iced::application(boot, update, view)
        .subscription(subscription)
        .theme(theme)
        .style(style)
        .window(window::Settings {
            size: Size::new(980.0, 640.0),
            min_size: Some(Size::new(760.0, 480.0)),
            transparent: true,
            blur: true,
            decorations: false, // Pure frameless! 100% black-border free!
            ..Default::default()
        })
        .run()
}
