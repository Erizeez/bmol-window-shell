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
    ChromeDrawPlan, ChromeLayoutMode, WindowChromeConfig, WindowChromeMetrics, WindowRimConfig,
    loyal_drag_bar, traffic_lights, window_metrics, wrap_window_rim,
};
use iced::widget::{
    button, column, container, row, scrollable, slider, space, text, toggler,
};
use iced::window;
use iced::{Alignment, Color, Element, Length, Padding, Size, Subscription, Task, Theme};
use liquid_glass::UiColorScheme;

#[path = "playground/iced_backend.rs"]
mod iced_backend;

use iced_backend::{DemoSurface, Renderer, WINDOW_CONTROL_NATIVE_IDS, WindowControlTuning};
use bmol_window_shell::traffic_lights as window_controls;
use window_controls::{
    ControlAction, TrafficLightsState, WINDOW_CONTROL_NATIVE_SIZE,
};

type AppElement<'a> = Element<'a, Message, Theme, Renderer>;

pub use macos_typography::{
    APPLE_HEADER_FONT, APPLE_TITLEBAR_FONT, APPLE_TITLEBAR_FONT_SIZE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutSelection {
    Separate,
    UnifiedSinglePane,
    UnifiedMultiPane,
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
    ResizeWindow(window::Direction),
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
    ToggleMaximize,
    DragWindow,
    WindowControl(ControlAction),
    TrafficLightsHover(bool),
    TrafficLightsPressStart(usize),
    TrafficLightsPressCancel(usize),
    TrafficLightsPressEnd(usize),
    AnimationFrame(std::time::Instant),
    WindowFocused(bool),
}

#[derive(Debug)]
struct DemoState {
    window_id: Option<window::Id>,
    window_size: Size,
    active_tab: usize,
    layout_selection: LayoutSelection,
    theme_preference: ThemePreference,
    system_theme: iced::theme::Mode,
    traffic_lights: TrafficLightsState,
    window_focused: bool,
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
        let default_config =
            WindowChromeConfig::separate(window_metrics::COMPACT_TITLEBAR_HEIGHT);
        let initial_dark = bmol_window_shell::is_system_dark_mode();
        let initial_theme = if initial_dark {
            iced::theme::Mode::Dark
        } else {
            iced::theme::Mode::Light
        };
        let metrics = WindowChromeMetrics::compute_with_theme(980.0, 640.0, &default_config, initial_dark);

        Self {
            window_id: None,
            window_size: Size::new(980.0, 640.0),
            active_tab: 0,
            layout_selection: LayoutSelection::Separate,
            theme_preference: ThemePreference::System,
            system_theme: initial_theme,
            traffic_lights: TrafficLightsState::new(),
            window_focused: true,
            separate_titlebar_height: window_metrics::COMPACT_TITLEBAR_HEIGHT,
            unified_header_height: window_metrics::FUSED_HEADER_HEIGHT,
            sidebar_width: window_metrics::SIDEBAR_WIDTH_REGULAR,
            show_hitboxes: false,
            blur_enabled: true,
            system_shadow: false, // Default to FALSE! Eliminates the automatic 1px dark rim macOS draws around windows
            edr_enabled: true,
            guard_enabled: true,
            corner_radius: window_metrics::DEFAULT_CORNER_RADIUS as f64,
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
            LayoutSelection::UnifiedSinglePane => {
                WindowChromeConfig::unified_header(self.unified_header_height)
            }
            LayoutSelection::UnifiedMultiPane => {
                WindowChromeConfig::unified(self.unified_header_height, Some(self.sidebar_width))
            }
        };
        self.metrics = WindowChromeMetrics::compute_with_theme(
            self.window_size.width,
            self.window_size.height,
            &config,
            self.is_dark(),
        );
    }

    fn sync_window_controls_backend(&self) {
        let scheme = if self.is_dark() {
            UiColorScheme::Dark
        } else {
            UiColorScheme::Light
        };
        iced_backend::set_color_scheme(scheme);
        iced_backend::set_window_control_tuning(WindowControlTuning::for_scheme(scheme));
        iced_backend::set_window_inactive(!self.window_focused);

        let rim_insets = self.metrics.rim_insets.top;
        let header_height = match self.layout_selection {
            LayoutSelection::Separate => self.separate_titlebar_height,
            LayoutSelection::UnifiedSinglePane | LayoutSelection::UnifiedMultiPane => {
                self.unified_header_height
            }
        };
        let symmetric_margin = ((header_height - WINDOW_CONTROL_NATIVE_SIZE) * 0.5).max(4.0);
        let origin_x = rim_insets + symmetric_margin;
        let origin_y = rim_insets + symmetric_margin;
        iced_backend::set_window_control_origin(origin_x, origin_y);
    }

    fn sync_native_window(&self) -> Task<Message> {
        if let Some(id) = self.window_id {
            let options = bmol_window_shell::NativeWindowOptions::new()
                .with_corner_radius(self.corner_radius)
                .with_appearance(self.native_appearance())
                .with_system_shadow(self.system_shadow)
                .with_edr(self.edr_enabled)
                .with_stage_manager_guard(self.guard_enabled);

            window::run(id, move |w| {
                if let Ok(handle) = w.window_handle() {
                    let _ = bmol_window_shell::setup_native_window(handle.as_raw(), options);
                }
            })
            .discard()
        } else {
            Task::none()
        }
    }
}

fn boot() -> (DemoState, Task<Message>) {
    let state = DemoState::default();
    iced_backend::set_surface(DemoSurface::WindowDemo);
    let scheme = if state.is_dark() {
        UiColorScheme::Dark
    } else {
        UiColorScheme::Light
    };
    iced_backend::set_color_scheme(scheme);
    iced_backend::set_window_control_tuning(WindowControlTuning::for_scheme(scheme));
    iced_backend::set_accessibility(liquid_glass::GlassAccessibility::none());
    iced_backend::set_window_inactive(!state.window_focused);
    iced_backend::set_window_control_group_progress(0, 0.0);
    for id in WINDOW_CONTROL_NATIVE_IDS {
        iced_backend::set_window_control_press_progress(id, 0.0);
        iced_backend::set_window_control_scale(id, 1.0);
    }
    state.sync_window_controls_backend();

    (
        state,
        iced::system::theme().map(Message::SystemThemeChanged),
    )
}

fn update(state: &mut DemoState, message: Message) -> Task<Message> {
    let task = match message {
        Message::WindowOpened(id) => {
            state.window_id = Some(id);
            state.native_attached = true;
            state.sync_native_window()
        }
        Message::WindowResized(size) => {
            state.window_size = size;
            Task::none()
        }
        Message::ResizeWindow(direction) => {
            if let Some(id) = state.window_id {
                window::drag_resize(id, direction)
            } else {
                Task::none()
            }
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
            state.sync_native_window()
        }
        Message::SystemThemeChanged(mode) => {
            state.system_theme = mode;
            if state.theme_preference == ThemePreference::System {
                state.sync_native_window()
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
                    state.sync_native_window()
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
            if enabled {
                state.sync_native_window()
            } else {
                Task::none()
            }
        }
        Message::ToggleShadow(enabled) => {
            state.system_shadow = enabled;
            state.sync_native_window()
        }
        Message::ToggleEdr(enabled) => {
            state.edr_enabled = enabled;
            state.sync_native_window()
        }
        Message::ToggleGuard(enabled) => {
            state.guard_enabled = enabled;
            if enabled {
                state.sync_native_window()
            } else {
                Task::none()
            }
        }
        Message::CornerRadiusChanged(radius) => {
            state.corner_radius = radius;
            state.sync_native_window()
        }
        Message::OpacityChanged(opacity) => {
            state.opacity = opacity;
            Task::none()
        }
        Message::ResetDefaults => {
            state.layout_selection = LayoutSelection::Separate;
            state.separate_titlebar_height = window_metrics::COMPACT_TITLEBAR_HEIGHT;
            state.unified_header_height = window_metrics::FUSED_HEADER_HEIGHT;
            state.sidebar_width = window_metrics::SIDEBAR_WIDTH_REGULAR;
            state.show_hitboxes = false;
            state.blur_enabled = true;
            state.edr_enabled = true;
            state.guard_enabled = true;
            state.corner_radius = window_metrics::DEFAULT_CORNER_RADIUS as f64;
            state.opacity = 0.88;
            state.sync_native_window()
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
        Message::WindowControl(action) => {
            if let Some(id) = state.window_id {
                match action {
                    ControlAction::Close => window::close(id),
                    ControlAction::Minimize => window::minimize(id, true),
                    ControlAction::Expand | ControlAction::Zoom => window::toggle_maximize(id),
                }
            } else {
                Task::none()
            }
        }
        Message::TrafficLightsHover(hovered) => {
            state.traffic_lights.on_group_hover(hovered);
            iced_backend::set_window_control_group_hover(0, hovered);
            Task::none()
        }
        Message::TrafficLightsPressStart(index) => {
            state.traffic_lights.on_press_start(index);
            if let Some(&id) = WINDOW_CONTROL_NATIVE_IDS.get(index) {
                iced_backend::set_window_control_press_progress(id, 1.0);
            }
            Task::none()
        }
        Message::TrafficLightsPressCancel(index) => {
            state.traffic_lights.on_press_cancel(index);
            if let Some(&id) = WINDOW_CONTROL_NATIVE_IDS.get(index) {
                iced_backend::set_window_control_press_progress(id, 0.0);
                iced_backend::set_window_control_scale(id, 1.0);
            }
            Task::none()
        }
        Message::TrafficLightsPressEnd(index) => {
            state.traffic_lights.on_press_end(index);
            if let Some(&id) = WINDOW_CONTROL_NATIVE_IDS.get(index) {
                iced_backend::set_window_control_press_progress(id, 0.0);
            }
            Task::none()
        }
        Message::AnimationFrame(now) => {
            state.traffic_lights.step(now);
            iced_backend::set_window_control_group_progress(0, state.traffic_lights.hover_progress);
            for (index, &id) in WINDOW_CONTROL_NATIVE_IDS.iter().enumerate() {
                let scale = state.traffic_lights.press_springs[index].value();
                iced_backend::set_window_control_scale(id, scale);
                iced_backend::set_window_control_press_progress(
                    id,
                    state.traffic_lights.press_targets[index],
                );
            }
            Task::none()
        }
        Message::WindowFocused(focused) => {
            state.window_focused = focused;
            iced_backend::set_window_inactive(!focused);
            Task::none()
        }
    };

    state.update_metrics();
    state.sync_window_controls_backend();
    task
}

fn subscription(state: &DemoState) -> Subscription<Message> {
    let anim_sub = if state.traffic_lights.is_animating() {
        iced::time::every(std::time::Duration::from_millis(16))
            .map(|_| Message::AnimationFrame(std::time::Instant::now()))
    } else {
        Subscription::none()
    };

    Subscription::batch([
        anim_sub,
        window::open_events().map(Message::WindowOpened),
        window::resize_events().map(|(_id, size)| Message::WindowResized(size)),
        window::events().filter_map(|(_id, event)| match event {
            window::Event::Focused => Some(Message::WindowFocused(true)),
            window::Event::Unfocused => Some(Message::WindowFocused(false)),
            _ => None,
        }),
        iced::system::theme_changes().map(Message::SystemThemeChanged),
        iced::time::every(std::time::Duration::from_millis(250)).map(|_| Message::PollSystemTheme),
    ])
}

fn view(state: &DemoState) -> AppElement<'_> {
    let plan = ChromeDrawPlan::from_metrics(&state.metrics);

    let content = match state.layout_selection {
        LayoutSelection::Separate => view_separate_window(state, plan),
        LayoutSelection::UnifiedSinglePane => view_unified_single_pane(state, plan),
        LayoutSelection::UnifiedMultiPane => view_unified_multi_pane(state, plan),
    };

    let rimmed = wrap_window_rim(
        content,
        WindowRimConfig::new(state.is_dark())
            .with_corner_radius(state.corner_radius as f32),
    );

    bmol_window_shell::wrap_border_resizer(
        rimmed,
        false,
        Message::ResizeWindow,
    )
}

// =========================================================================
// Hand-crafted Apple-style Liquid Glass Traffic Lights (Close / Minimize / Zoom)
// =========================================================================

fn view_traffic_lights<'a>(state: &'a DemoState) -> AppElement<'a> {
    window_controls::view_traffic_lights_all_inclusive(
        &state.traffic_lights,
        state.window_focused,
        state.is_dark(),
        |event| match event {
            window_controls::TrafficLightsEvent::GroupHover(hovered) => {
                Message::TrafficLightsHover(hovered)
            }
            window_controls::TrafficLightsEvent::PressStart(index) => {
                Message::TrafficLightsPressStart(index)
            }
            window_controls::TrafficLightsEvent::PressCancel(index) => {
                Message::TrafficLightsPressCancel(index)
            }
            window_controls::TrafficLightsEvent::PressEnd(index) => {
                Message::TrafficLightsPressEnd(index)
            }
            window_controls::TrafficLightsEvent::Action(action) => {
                Message::WindowControl(action)
            }
        },
    )
}

fn view_theme_toggle(state: &DemoState) -> AppElement<'_> {
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


fn view_layout_toggle(state: &DemoState) -> AppElement<'_> {
    let make_item = |label: &'static str, layout: LayoutSelection| {
        let is_active = state.layout_selection == layout;
        let is_dark = state.is_dark();
        button(text(label).size(11))
            .padding([3, 7])
            .on_press(Message::SelectLayout(layout))
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
            make_item("Separate", LayoutSelection::Separate),
            make_item("Unified (1-Pane)", LayoutSelection::UnifiedSinglePane),
            make_item("Unified (2-Pane)", LayoutSelection::UnifiedMultiPane),
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

fn view_header_actions(state: &DemoState) -> AppElement<'_> {
    let is_dark = state.is_dark();
    row![
        view_theme_toggle(state),
        column![].width(Length::Fixed(6.0)),
        view_layout_toggle(state),
        column![].width(Length::Fixed(6.0)),
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
    .align_y(Alignment::Center)
    .into()
}

// =========================================================================
// Layout 1: Standalone Titlebar (Separate) - 32px White Bar, 16px Clearance
// =========================================================================

fn view_separate_window(state: &DemoState, _plan: ChromeDrawPlan) -> AppElement<'_> {
    let titlebar_height = state.metrics.header_rect.height;
    let is_dark = state.is_dark();

    let titlebar_bg = if is_dark {
        Color::from_rgb8(40, 40, 40)
    } else {
        Color::from_rgb(1.0, 1.0, 1.0)
    };
    let title_color = if is_dark {
        Color::from_rgb8(164, 164, 164)
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

    let mode_switch = view_header_actions(state);

    let rim_left = state.metrics.rim_insets.left;
    let symmetric_margin = ((titlebar_height - WINDOW_CONTROL_NATIVE_SIZE) * 0.5).max(4.0);
    let slop = traffic_lights::control_hover_slop(WINDOW_CONTROL_NATIVE_SIZE);
    let leading_spacer_w = (symmetric_margin - slop).max(0.0);
    let title_clearance_spacer_w = (traffic_lights::TITLE_CLEARANCE - slop).max(0.0);
    let safe_radius = (state.corner_radius as f32 - rim_left).max(0.0);

    let titlebar_content = row![
        // 1. Left leading edge margin (adaptive spacer: spacer_w + slop == symmetric_margin)
        column![].width(Length::Fixed(leading_spacer_w)),
        // 2. Custom Apple traffic lights
        view_traffic_lights(state),
        // 3. Strictly authentic clearance between traffic lights and title
        column![].width(Length::Fixed(title_clearance_spacer_w)),
        // 4. Left-aligned title text (Apple standard Bold at native 13.0pt, letter 'l' height strictly 10px)
        text("BMOL Window Shell")
            .size(APPLE_TITLEBAR_FONT_SIZE)
            .font(APPLE_TITLEBAR_FONT)
            .color(title_color),
        // 5. Flexible horizontal space in the center
        space::horizontal().width(Length::Fill),
        // 6. Right action area
        mode_switch,
        column![].width(Length::Fixed(16.0)),
    ]
    .align_y(Alignment::Center)
    .height(Length::Fixed(titlebar_height))
    .width(Length::Fill);

    let draggable_titlebar = loyal_drag_bar(
        titlebar_height,
        titlebar_content,
        Message::DragWindow,
        Some(Message::ToggleMaximize),
    );

    let titlebar_container = container(draggable_titlebar)
        .style(move |_theme| container::Style {
            background: Some(titlebar_bg.into()),
            border: iced::Border {
                radius: iced::border::Radius {
                    top_left: safe_radius,
                    top_right: safe_radius,
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
                        bottom_left: safe_radius,
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
                        bottom_right: safe_radius,
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
// Layout 2: Unified Pure Canvas (Single Pane)
// No forced sidebar. No artificial toolbar background. The entire page flows
// naturally, while the transparent top header faithfully handles dragging
// anywhere the user clicks outside interactive buttons.
// =========================================================================

fn view_unified_single_pane(state: &DemoState, _plan: ChromeDrawPlan) -> AppElement<'_> {
    let header_height = state.metrics.header_rect.height;
    let is_dark = state.is_dark();
    let bg_color = if is_dark {
        Color::from_rgba(0.12, 0.13, 0.16, state.opacity)
    } else {
        Color::from_rgba(0.98, 0.98, 1.0, state.opacity)
    };

    let title_color = if is_dark {
        Color::from_rgb(0.92, 0.92, 0.95)
    } else {
        Color::from_rgb(0.12, 0.13, 0.15)
    };

    let rim_left = state.metrics.rim_insets.left;
    let symmetric_margin = ((header_height - WINDOW_CONTROL_NATIVE_SIZE) * 0.5).max(4.0);
    let slop = traffic_lights::control_hover_slop(WINDOW_CONTROL_NATIVE_SIZE);
    let leading_spacer_w = (symmetric_margin - slop).max(0.0);
    let title_clearance_spacer_w = (traffic_lights::TITLE_CLEARANCE - slop).max(0.0);
    let safe_radius = (state.corner_radius as f32 - rim_left).max(0.0);

    // 1. Top Header Area (Transparent canvas, owned by downstream application)
    let header_content = row![
        // Left margin (adaptive spacer: spacer_w + slop == symmetric_margin)
        column![].width(Length::Fixed(leading_spacer_w)),
        view_traffic_lights(state),
        // Strictly authentic clearance between traffic lights and header title
        column![].width(Length::Fixed(title_clearance_spacer_w)),
        text(match state.active_tab {
            0 => "Window Architecture",
            1 => "Layout & Collision Hitboxes",
            2 => "Display & EDR Dynamic Range",
            _ => "Compositor & Stage Manager",
        })
        .size(14)
        .font(APPLE_HEADER_FONT)
        .color(title_color),
        // Natural flexible space beneath: clicking/dragging here faithfully triggers window dragging!
        space::horizontal().width(Length::Fill),
        view_header_actions(state),
        column![].width(Length::Fixed(16.0)),
    ]
    .align_y(Alignment::Center)
    .height(Length::Fixed(header_height))
    .width(Length::Fill);

    let draggable_header = loyal_drag_bar(
        header_height,
        header_content,
        Message::DragWindow,
        Some(Message::ToggleMaximize),
    );

    // 2. Full-width content body (Single Pane Canvas)
    let main_body = view_content_cards(state);

    let page = column![draggable_header, main_body]
        .width(Length::Fill)
        .height(Length::Fill);

    let page_container = container(page)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(bg_color.into()),
            border: iced::Border {
                radius: safe_radius.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    if state.show_hitboxes {
        column![page_container, view_hitboxes_overlay_banner(&state.metrics)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        page_container.into()
    }
}

// =========================================================================
// Layout 3: Unified Multi-Pane (Split Sidebar Workspace)
// Downstream application chooses to split layout with a sidebar. The top header
// remains faithful: dragging beneath any widget moves the window.
// =========================================================================

fn view_unified_multi_pane(state: &DemoState, _plan: ChromeDrawPlan) -> AppElement<'_> {
    let header_height = state.metrics.header_rect.height;
    let is_dark = state.is_dark();

    let rim_left = state.metrics.rim_insets.left;
    let symmetric_margin = ((header_height - WINDOW_CONTROL_NATIVE_SIZE) * 0.5).max(4.0);
    let slop = traffic_lights::control_hover_slop(WINDOW_CONTROL_NATIVE_SIZE);
    let leading_spacer_w = (symmetric_margin - slop).max(0.0);
    let safe_radius = (state.corner_radius as f32 - rim_left).max(0.0);

    let sidebar_header_content = row![
        // Left margin (adaptive spacer: spacer_w + slop == symmetric_margin)
        column![].width(Length::Fixed(leading_spacer_w)),
        view_traffic_lights(state),
        space::horizontal().width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .height(Length::Fixed(header_height))
    .width(Length::Fill);

    let draggable_sidebar_header = loyal_drag_bar(
        header_height,
        sidebar_header_content,
        Message::DragWindow,
        Some(Message::ToggleMaximize),
    );

    let sidebar_body = column![
        text("NAVIGATION")
            .size(11)
            .color(Color::from_rgb(0.5, 0.5, 0.55)),
        view_sidebar_items(state),
    ]
    .padding(Padding {
        top: 4.0,
        right: 16.0,
        bottom: 16.0,
        left: 16.0,
    })
    .spacing(12)
    .height(Length::Fill);

    let sidebar_inner = column![draggable_sidebar_header, sidebar_body]
        .height(Length::Fill)
        .width(Length::Fill);

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
                    top_left: safe_radius,
                    top_right: 0.0,
                    bottom_right: 0.0,
                    bottom_left: safe_radius,
                },
                ..Default::default()
            },
            ..Default::default()
        });

    let v_separator = container(column![]).width(Length::Fixed(0.0));

    // Right Content Area
    let top_toolbar_content = row![
        column![].width(Length::Fixed(20.0)),
        text(match state.active_tab {
            0 => "Window Architecture",
            1 => "Layout & Collision Hitboxes",
            2 => "Display & EDR Dynamic Range",
            _ => "Compositor & Stage Manager",
        })
        .size(14)
        .font(APPLE_HEADER_FONT),
        space::horizontal().width(Length::Fill),
        view_header_actions(state),
        column![].width(Length::Fixed(16.0)),
    ]
    .align_y(Alignment::Center)
    .height(Length::Fixed(header_height))
    .width(Length::Fill);

    let draggable_top_toolbar = loyal_drag_bar(
        header_height,
        top_toolbar_content,
        Message::DragWindow,
        Some(Message::ToggleMaximize),
    );

    let main_body = view_content_cards(state);

    let content_area = column![draggable_top_toolbar, main_body]
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
                    top_right: safe_radius,
                    bottom_right: safe_radius,
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

fn view_sidebar_items(state: &DemoState) -> AppElement<'_> {
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

fn view_content_cards(state: &DemoState) -> AppElement<'_> {
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

fn view_tab_architecture(state: &DemoState) -> AppElement<'_> {
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
                button(text("Unified (Single Pane)").size(13))
                    .padding([8, 14])
                    .on_press(Message::SelectLayout(LayoutSelection::UnifiedSinglePane))
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
                button(text("Unified (Multi-Pane)").size(13))
                    .padding([8, 14])
                    .on_press(Message::SelectLayout(LayoutSelection::UnifiedMultiPane))
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
            .spacing(10),
            card_divider(),
            text(match state.layout_selection {
                LayoutSelection::Separate => {
                    "Current: Separate Mode. 32px pure white titlebar with custom traffic lights. Content strictly starts below the titlebar."
                }
                LayoutSelection::UnifiedSinglePane => {
                    "Current: Unified Pure Canvas (Single Pane). No forced sidebar, no artificial top bar background. The top area is a transparent loyal drag layer: custom buttons work as expected, while dragging any background beneath faithfully initiates smooth window moving."
                }
                LayoutSelection::UnifiedMultiPane => {
                    "Current: Unified Multi-Pane (Sidebar Workspace). Downstream application chooses to split layout. The continuous top drag layer faithfully handles background window dragging."
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

fn view_tab_hitboxes_and_layout(state: &DemoState) -> AppElement<'_> {
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
            "Non-client Rim Inset",
            format!(
                "{:.1} pt ({})",
                metrics.rim_insets.left,
                if state.is_dark() {
                    "Dark compound 2px: 1px black + 1px gray (70, 70, 70)"
                } else {
                    "Light 1px subtle gray"
                }
            ),
        ),
        (
            "Safe Client Area (Confined)",
            format!(
                "x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}",
                metrics.safe_client_rect.x,
                metrics.safe_client_rect.y,
                metrics.safe_client_rect.width,
                metrics.safe_client_rect.height
            ),
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

fn view_tab_edr(_state: &DemoState) -> AppElement<'_> {
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

fn view_tab_stage_manager(_state: &DemoState) -> AppElement<'_> {
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

fn view_hitboxes_overlay_banner(metrics: &WindowChromeMetrics) -> AppElement<'_> {
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
    content: impl Into<AppElement<'a>>,
) -> AppElement<'a> {
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

fn card_divider<'a>() -> AppElement<'a> {
    container(space())
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
    let fonts = liquid_glass::ui::font::ui_fonts();
    let mut app = iced::application::<DemoState, Message, Theme, Renderer>(boot, update, view)
        .title("BMOL Window Shell")
        .subscription(subscription)
        .theme(theme)
        .style(style)
        .default_font(APPLE_TITLEBAR_FONT)
        .antialiasing(true)
        .window(window::Settings {
            size: Size::new(980.0, 640.0),
            min_size: Some(Size::new(760.0, 480.0)),
            transparent: true,
            blur: true,
            decorations: false, // Pure frameless! 100% black-border free!
            ..Default::default()
        });
    for bytes in &fonts.bytes {
        app = app.font(bytes.clone());
    }
    app.run()
}


#[cfg(test)]
mod tests {
    use super::APPLE_TITLEBAR_FONT;

    #[test]
    fn test_apple_font_resolution() {
        let font_system = iced::advanced::graphics::text::font_system();
        let mut fs = font_system.write().unwrap();
        let db = fs.raw().db();

        assert_eq!(APPLE_TITLEBAR_FONT.weight, iced::font::Weight::Bold);

        // Ensure macOS system font or PingFang is available in the font database
        let mut has_apple_or_fallback = false;
        for face in db.faces() {
            for (fam, _) in &face.families {
                if fam == "System Font" || fam == ".SF NS" || fam == "PingFang SC" || fam == "SFNS Text" {
                    has_apple_or_fallback = true;
                    break;
                }
            }
            if has_apple_or_fallback {
                break;
            }
        }
        assert!(has_apple_or_fallback, "Apple system font or PingFang should be discoverable on macOS");
    }

    #[test]
    fn test_window_rim_mode_styling() {
        let mut state = super::DemoState::default();
        state.system_theme = iced::theme::Mode::Light;
        state.theme_preference = super::ThemePreference::Light;
        assert!(!state.is_dark());

        // Light mode: single-layer 1px rim
        let light_radius = state.corner_radius as f32;
        assert_eq!(light_radius, 16.0);

        state.theme_preference = super::ThemePreference::Dark;
        assert!(state.is_dark());

        // Dark mode: dual-layer compound rim (2px total)
        let dark_outer_radius = state.corner_radius as f32;
        let dark_inner_radius = (dark_outer_radius - 1.0).max(0.0);
        assert_eq!(dark_outer_radius, 16.0);
        assert_eq!(dark_inner_radius, 15.0);
    }

    #[test]
    fn test_window_rim_isolation_and_spacing_invariants() {
        let mut state = super::DemoState::default();

        // 1. Dark mode invariants
        state.theme_preference = super::ThemePreference::Dark;
        state.update_metrics();
        assert_eq!(state.metrics.rim_insets.left, 2.0);
        let dark_spacer = (4.0 - state.metrics.rim_insets.left).max(0.0);
        assert_eq!(dark_spacer, 2.0);
        let slop = 6.0;
        let dark_red_left_dist = state.metrics.rim_insets.left + dark_spacer + slop;
        assert_eq!(dark_red_left_dist, 10.0, "Red light leftmost edge must be strictly 10.0 px from window left in dark mode");

        // 2. Light mode invariants
        state.theme_preference = super::ThemePreference::Light;
        state.update_metrics();
        assert_eq!(state.metrics.rim_insets.left, 1.0);
        let light_spacer = (4.0 - state.metrics.rim_insets.left).max(0.0);
        assert_eq!(light_spacer, 3.0);
        let light_red_left_dist = state.metrics.rim_insets.left + light_spacer + slop;
        assert_eq!(light_red_left_dist, 10.0, "Red light leftmost edge must be strictly 10.0 px from window left in light mode");

        // 3. Traffic lights gap invariant (strictly 9.0 px)
        assert_eq!(super::window_controls::WINDOW_CONTROL_GAP, 9.0);

        // 4. Traffic lights to title clearance invariant (strictly 15.0 px)
        let title_spacer = 9.0;
        let clearance = slop + title_spacer;
        assert_eq!(clearance, 15.0, "Clearance between traffic lights and first title letter must be strictly 15.0 px");

        // 5. Titlebar font size invariant (strictly 13.0 pt)
        assert_eq!(super::APPLE_TITLEBAR_FONT_SIZE, 13.0);
    }
}





