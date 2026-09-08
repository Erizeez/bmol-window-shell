//! Authentic macOS-style window controls (Traffic Lights) for bmol-window-shell.
//!
//! Provides pixel-perfect Close, Minimize, and Zoom/Fullscreen buttons
//! adhering to Apple macOS Human Interface Guidelines (HIG):
//! - Strict 14.0 pt diameter and 9.0 pt spacing (total width 60.0 pt).
//! - True Apple spring-driven physics (overshoot 1.18, settle 1.06, bouncy timing).
//! - Smooth exponential hover transition (enter 0.08s, exit 0.18s).
//! - Authentic Apple vector SVG glyphs (exact extracted SF cross, minus bar, dual-triangle zoom).
//! - True Apple colorimetry with subtle 0.5 pt boundary rim delineation and 0.5 pt drop shadow.
//! - Inactive/unfocused window desaturation with smooth hover fade-in.
//! - Optical slop tracking area to avoid hover flickering between buttons.

use std::time::Instant;

use iced::advanced::{self, svg as advanced_svg, text as advanced_text};
use iced::widget::{container, mouse_area, row, space, svg};
use iced::{Background, Color, Element, Length, Padding, Vector};
use spring_rs::{Spring, SpringMotion};

pub use crate::platform::traffic_lights::*;
use crate::platform::traffic_lights as metrics;

// Authentic Apple Vector SVGs
pub const SVG_CLOSE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.2" d="M1.5 1.5L8.5 8.5M8.5 1.5L1.5 8.5"/></svg>"#;
pub const SVG_MINIMIZE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 4"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="1.2" d="M1.0 2.0h8.0"/></svg>"#;
pub const SVG_ZOOM: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-0.250 114.188 46.031 46.062"><path fill="currentColor" fill-rule="nonzero" d=" M 0.000 122.313 L 0.000 146.688 C 0.000 149.063 2.312 149.875 3.750 148.438 L 34.000 118.188 C 35.406 116.781 34.594 114.438 32.250 114.438 L 7.969 114.438 C 2.656 114.438 0.000 117.063 0.000 122.313 Z M 37.687 160.000 C 42.937 160.000 45.531 157.313 45.531 152.031 L 45.531 127.750 C 45.531 125.375 43.219 124.563 41.781 126.000 L 11.531 156.250 C 10.125 157.656 10.937 160.000 13.281 160.000 Z"/></svg>"#;
pub const SVG_MAXIMIZE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="1.2" d="M5 1.5v7M1.5 5h7"/></svg>"#;

// Standard macOS traffic light dimensions and aliases
pub const WINDOW_CONTROL_NATIVE_SIZE: f32 = 14.0;
pub const WINDOW_CONTROL_LARGE_SIZE: f32 = 64.0;
pub const WINDOW_CONTROL_GAP: f32 = 9.0;
pub const WINDOW_CONTROL_LARGE_GAP: f32 = 12.0;
pub const WINDOW_CONTROL_NATIVE_IDS: [usize; 3] = [0, 1, 2];

// Spring physics constants matching authentic macOS click response
pub const PRESS_SCALE_OVERSHOOT: f32 = 1.18;
pub const PRESS_SCALE_SETTLED: f32 = 1.06;
pub const PRESS_SCALE_SPRING_DURATION: f32 = 0.24;
pub const PRESS_SCALE_SPRING_EXTRA_BOUNCE: f32 = 0.40;
pub const INTERACTION_ENTER_ANIMATION_TIME_CONSTANT: f32 = 0.08;
pub const INTERACTION_EXIT_ANIMATION_TIME_CONSTANT: f32 = 0.18;

/// Semantic window control actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowControlAction {
    Close,
    Minimize,
    Zoom,
    Expand,
}

/// Backwards-compatible alias for WindowControlAction.
pub type ControlAction = WindowControlAction;

impl WindowControlAction {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Close => "Close",
            Self::Minimize => "Minimize",
            Self::Zoom | Self::Expand => "Zoom",
        }
    }
}

/// The semantic action represented by a macOS-style green traffic light.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum WindowExpandBehavior {
    /// Enter true fullscreen and return to windowed mode on the next press.
    #[default]
    Fullscreen,
    /// Maximize to screen visible frame (classic zoom).
    Zoom,
    /// Maximize into the current work area and restore the previous frame.
    Maximize,
}

impl WindowExpandBehavior {
    #[must_use]
    pub const fn is_fullscreen(self) -> bool {
        matches!(self, Self::Fullscreen)
    }
}

/// Returns the matching Apple vector SVG source for the action.
#[must_use]
pub const fn window_control_icon(
    action: WindowControlAction,
    expand_behavior: WindowExpandBehavior,
) -> &'static str {
    match action {
        WindowControlAction::Close => SVG_CLOSE,
        WindowControlAction::Minimize => SVG_MINIMIZE,
        WindowControlAction::Zoom | WindowControlAction::Expand => {
            if expand_behavior.is_fullscreen() {
                SVG_ZOOM
            } else {
                SVG_MAXIMIZE
            }
        }
    }
}

/// Control group variants for inspection and showcase.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ControlGroup {
    Native,
    Reference,
    Active,
    Inactive,
    Disabled,
}

impl ControlGroup {
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Native => 0,
            Self::Reference => 1,
            Self::Active => 2,
            Self::Inactive => 3,
            Self::Disabled => 4,
        }
    }
}

/// Dynamic interaction and animation state for traffic lights (springs, hover decay, timer).
#[derive(Debug, Clone)]
pub struct TrafficLightsState {
    pub hover_progress: f32,
    pub hover_target: f32,
    pub press_springs: [SpringMotion; 3],
    pub press_targets: [f32; 3],
    pub expand_behavior: WindowExpandBehavior,
    pub last_tick: Option<Instant>,
}

impl Default for TrafficLightsState {
    fn default() -> Self {
        Self {
            hover_progress: 0.0,
            hover_target: 0.0,
            press_springs: [
                SpringMotion::new(
                    1.0,
                    1.0,
                    Spring::bouncy_custom(
                        PRESS_SCALE_SPRING_DURATION,
                        PRESS_SCALE_SPRING_EXTRA_BOUNCE,
                    ),
                ),
                SpringMotion::new(
                    1.0,
                    1.0,
                    Spring::bouncy_custom(
                        PRESS_SCALE_SPRING_DURATION,
                        PRESS_SCALE_SPRING_EXTRA_BOUNCE,
                    ),
                ),
                SpringMotion::new(
                    1.0,
                    1.0,
                    Spring::bouncy_custom(
                        PRESS_SCALE_SPRING_DURATION,
                        PRESS_SCALE_SPRING_EXTRA_BOUNCE,
                    ),
                ),
            ],
            press_targets: [0.0; 3],
            expand_behavior: WindowExpandBehavior::Fullscreen,
            last_tick: None,
        }
    }
}

impl TrafficLightsState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_group_hover(&mut self, hovered: bool) {
        self.hover_target = if hovered { 1.0 } else { 0.0 };
        if self.last_tick.is_none() {
            self.last_tick = Some(Instant::now());
        }
    }

    pub fn on_press_start(&mut self, index: usize) {
        if index < 3 {
            self.press_targets[index] = 1.0;
            self.press_springs[index].retarget(PRESS_SCALE_OVERSHOOT);
            if self.last_tick.is_none() {
                self.last_tick = Some(Instant::now());
            }
        }
    }

    pub fn on_press_end(&mut self, index: usize) {
        if index < 3 {
            self.press_targets[index] = 0.0;
            self.press_springs[index].retarget(PRESS_SCALE_SETTLED);
            if self.last_tick.is_none() {
                self.last_tick = Some(Instant::now());
            }
        }
    }

    pub fn on_press_cancel(&mut self, index: usize) {
        if index < 3 {
            self.press_targets[index] = 0.0;
            self.press_springs[index].retarget(1.0);
            if self.last_tick.is_none() {
                self.last_tick = Some(Instant::now());
            }
        }
    }

    pub fn step(&mut self, now: Instant) {
        let dt = if let Some(last) = self.last_tick {
            let elapsed = now.saturating_duration_since(last).as_secs_f32();
            if elapsed > 0.0 {
                elapsed.min(0.1)
            } else {
                0.016
            }
        } else {
            0.016
        };
        self.last_tick = Some(now);

        let hover_tc = if self.hover_target >= self.hover_progress {
            INTERACTION_ENTER_ANIMATION_TIME_CONSTANT
        } else {
            INTERACTION_EXIT_ANIMATION_TIME_CONSTANT
        };
        let hover_step = 1.0 - (-dt / hover_tc).exp();
        self.hover_progress += (self.hover_target - self.hover_progress) * hover_step;
        if (self.hover_target - self.hover_progress).abs() < 0.001 {
            self.hover_progress = self.hover_target;
        }

        for (spring, target) in self.press_springs.iter_mut().zip(self.press_targets.iter()) {
            spring.step(dt);
            if spring.is_settled(0.001, 0.01) && (*target - spring.value()).abs() < 0.001 {
                spring.retarget(*target);
            }
        }

        if !self.is_animating() {
            self.last_tick = None;
        }
    }

    #[must_use]
    pub fn is_animating(&self) -> bool {
        (self.hover_target - self.hover_progress).abs() > 0.001
            || self.press_springs.iter().any(|s| !s.is_settled(0.001, 0.01))
    }
}

/// Visual and interaction configuration for rendering traffic lights.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrafficLightsViewConfig {
    pub is_focused: bool,
    pub is_dark: bool,
    pub hover_progress: f32,
    pub press_scales: [f32; 3],
    pub is_fullscreen_symbol: bool,
}

impl Default for TrafficLightsViewConfig {
    fn default() -> Self {
        Self {
            is_focused: true,
            is_dark: false,
            hover_progress: 0.0,
            press_scales: [1.0, 1.0, 1.0],
            is_fullscreen_symbol: true,
        }
    }
}

impl TrafficLightsViewConfig {
    #[must_use]
    pub const fn new(is_focused: bool, is_dark: bool) -> Self {
        Self {
            is_focused,
            is_dark,
            hover_progress: 0.0,
            press_scales: [1.0, 1.0, 1.0],
            is_fullscreen_symbol: true,
        }
    }

    #[must_use]
    pub fn from_state(state: &TrafficLightsState, is_focused: bool, is_dark: bool) -> Self {
        Self {
            is_focused,
            is_dark,
            hover_progress: state.hover_progress,
            press_scales: [
                state.press_springs[0].value(),
                state.press_springs[1].value(),
                state.press_springs[2].value(),
            ],
            is_fullscreen_symbol: state.expand_behavior == WindowExpandBehavior::Fullscreen,
        }
    }

    #[must_use]
    pub const fn with_hover(mut self, progress: f32) -> Self {
        self.hover_progress = progress;
        self
    }

    #[must_use]
    pub const fn with_press_scales(mut self, scales: [f32; 3]) -> Self {
        self.press_scales = scales;
        self
    }

    #[must_use]
    pub const fn with_fullscreen_symbol(mut self, fullscreen: bool) -> Self {
        self.is_fullscreen_symbol = fullscreen;
        self
    }
}

/// Helper to blend two colors linearly.
#[must_use]
pub fn blend_color(from: Color, to: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color::from_rgba(
        from.r + (to.r - from.r) * amount,
        from.g + (to.g - from.g) * amount,
        from.b + (to.b - from.b) * amount,
        from.a + (to.a - from.a) * amount,
    )
}

/// Glyph size inside a control circle of `size`.
#[must_use]
pub fn window_control_glyph_size(action: WindowControlAction, size: f32) -> f32 {
    let factor = if size <= WINDOW_CONTROL_NATIVE_SIZE {
        match action {
            WindowControlAction::Close => 7.0 / WINDOW_CONTROL_NATIVE_SIZE,
            WindowControlAction::Minimize => 8.0 / WINDOW_CONTROL_NATIVE_SIZE,
            WindowControlAction::Zoom | WindowControlAction::Expand => 0.42,
        }
    } else {
        0.42
    };
    (size * factor).max(4.0)
}

/// Color for the control glyph based on action, is_dark, inactive state, and focus amount.
#[must_use]
pub fn window_control_glyph_color(
    is_dark: bool,
    action: WindowControlAction,
    inactive: bool,
    focus_amount: f32,
) -> Color {
    let focus_amount = focus_amount.clamp(0.0, 1.0);
    let active = match action {
        WindowControlAction::Close => Color::from_rgb8(0x4C, 0x00, 0x00),
        WindowControlAction::Minimize => Color::from_rgb8(0x5A, 0x36, 0x00),
        WindowControlAction::Zoom | WindowControlAction::Expand => Color::from_rgb8(0x0A, 0x38, 0x00),
    };
    let inactive_color = if is_dark {
        Color::from_rgba(0.82, 0.83, 0.86, 0.76)
    } else {
        Color::from_rgba(0.52, 0.53, 0.56, 0.78)
    };
    let color = if inactive {
        blend_color(inactive_color, active, focus_amount)
    } else {
        active
    };
    let alpha = (if is_dark { 0.85 } else { 0.75 }) * focus_amount;
    Color::from_rgba(color.r, color.g, color.b, alpha)
}

/// Status dot for disabled or unsaved close control.
#[must_use]
pub fn window_control_status_dot<'a, Message: 'a, Theme: 'a + container::Catalog, Renderer: advanced::Renderer + 'a>(
    color: Color,
    size: f32,
) -> Element<'a, Message, Theme, Renderer>
where
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
{
    let dot_size = (size * 0.24).max(3.0);
    container(space())
        .width(Length::Fixed(dot_size))
        .height(Length::Fixed(dot_size))
        .style(move |_theme| container::Style {
            background: Some(Background::Color(color)),
            border: iced::Border::default().rounded(dot_size * 0.5),
            ..Default::default()
        })
        .into()
}

/// Centered container helper for optical alignment.
#[must_use]
pub fn centered<'a, Message: 'a, Theme: 'a + container::Catalog, Renderer: advanced::Renderer + 'a>(
    content: Element<'a, Message, Theme, Renderer>,
    size: f32,
) -> Element<'a, Message, Theme, Renderer> {
    container(content)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .into()
}

/// Resolves fill and border colors for an active button.
#[must_use]
pub fn resolve_active_button_colors(
    action: WindowControlAction,
    is_dark: bool,
    is_pressed: bool,
    is_hovered: bool,
) -> (Color, Color) {
    if is_dark {
        match action {
            WindowControlAction::Close => {
                let fill = if is_pressed {
                    Color::from_rgb8(0xD3, 0x3B, 0x36)
                } else if is_hovered {
                    Color::from_rgb8(0xFF, 0x6E, 0x67)
                } else {
                    Color::from_rgb8(0xFF, 0x5F, 0x56)
                };
                (fill, Color::from_rgb8(0xB8, 0x32, 0x2B))
            }
            WindowControlAction::Minimize => {
                let fill = if is_pressed {
                    Color::from_rgb8(0xD7, 0x96, 0x1E)
                } else if is_hovered {
                    Color::from_rgb8(0xFF, 0xC8, 0x47)
                } else {
                    Color::from_rgb8(0xFF, 0xBD, 0x2E)
                };
                (fill, Color::from_rgb8(0xC2, 0x82, 0x16))
            }
            WindowControlAction::Zoom | WindowControlAction::Expand => {
                let fill = if is_pressed {
                    Color::from_rgb8(0x19, 0xA0, 0x23)
                } else if is_hovered {
                    Color::from_rgb8(0x32, 0xD8, 0x4D)
                } else {
                    Color::from_rgb8(0x27, 0xC9, 0x3F)
                };
                (fill, Color::from_rgb8(0x14, 0x8C, 0x1C))
            }
        }
    } else {
        match action {
            WindowControlAction::Close => {
                let fill = if is_pressed {
                    Color::from_rgb8(0xD2, 0x3C, 0x34)
                } else if is_hovered {
                    Color::from_rgb8(0xFF, 0x6E, 0x67)
                } else {
                    Color::from_rgb8(0xFF, 0x5F, 0x56)
                };
                (fill, Color::from_rgb8(0xE0, 0x44, 0x3E))
            }
            WindowControlAction::Minimize => {
                let fill = if is_pressed {
                    Color::from_rgb8(0xD7, 0x96, 0x1E)
                } else if is_hovered {
                    Color::from_rgb8(0xFF, 0xC8, 0x47)
                } else {
                    Color::from_rgb8(0xFF, 0xBD, 0x2E)
                };
                (fill, Color::from_rgb8(0xDE, 0xA1, 0x23))
            }
            WindowControlAction::Zoom | WindowControlAction::Expand => {
                let fill = if is_pressed {
                    Color::from_rgb8(0x19, 0xA0, 0x23)
                } else if is_hovered {
                    Color::from_rgb8(0x32, 0xD8, 0x4D)
                } else {
                    Color::from_rgb8(0x27, 0xC9, 0x3F)
                };
                (fill, Color::from_rgb8(0x1A, 0xAB, 0x29))
            }
        }
    }
}

/// Resolves fill and border colors for an inactive button.
#[must_use]
pub fn resolve_inactive_button_colors(is_dark: bool) -> (Color, Color) {
    if is_dark {
        (
            Color::from_rgb8(0x4C, 0x4C, 0x50),
            Color::from_rgba(0.20, 0.20, 0.22, 0.60),
        )
    } else {
        (
            Color::from_rgb8(0xD1, 0xD1, 0xD6),
            Color::from_rgba(0.70, 0.70, 0.73, 0.80),
        )
    }
}

/// Resolves fill and border colors for a button action and state.
#[must_use]
pub fn resolve_button_colors(
    action: WindowControlAction,
    is_dark: bool,
    is_active: bool,
    is_pressed: bool,
    is_hovered: bool,
) -> (Color, Color) {
    if !is_active {
        resolve_inactive_button_colors(is_dark)
    } else {
        resolve_active_button_colors(action, is_dark, is_pressed, is_hovered)
    }
}

/// Builds a single authentic macOS traffic light button.
pub fn view_single_button<'a, Message: Clone + 'a, Theme: 'a + container::Catalog + svg::Catalog, Renderer>(
    action: WindowControlAction,
    size: f32,
    is_dark: bool,
    is_active: bool,
    hover_progress: f32,
    scale: f32,
    is_fullscreen_symbol: bool,
    on_action: Message,
) -> Element<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer + advanced_text::Renderer + advanced_svg::Renderer + 'a,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    <Theme as svg::Catalog>::Class<'a>: From<svg::StyleFn<'a, Theme>>,
{
    let visual_size = size * scale;
    let hover = hover_progress.clamp(0.0, 1.0);
    let is_pressed = scale > 1.03;
    let is_hovered = hover > 0.5;

    let (active_fill, active_border) =
        resolve_active_button_colors(action, is_dark, is_pressed, is_hovered);
    let (fill, border) = if is_active {
        (active_fill, active_border)
    } else if hover > 0.0 {
        let (inactive_fill, inactive_border) = resolve_inactive_button_colors(is_dark);
        (
            blend_color(inactive_fill, active_fill, hover),
            blend_color(inactive_border, active_border, hover),
        )
    } else {
        resolve_inactive_button_colors(is_dark)
    };

    let glyph_widget: Element<'a, Message, Theme, Renderer> = if hover < 0.01 {
        space()
            .width(Length::Fixed(visual_size))
            .height(Length::Fixed(visual_size))
            .into()
    } else {
        let glyph_color = window_control_glyph_color(is_dark, action, !is_active, hover);
        let glyph_size = window_control_glyph_size(action, size) * scale;
        let svg_str = match action {
            WindowControlAction::Close => SVG_CLOSE,
            WindowControlAction::Minimize => SVG_MINIMIZE,
            WindowControlAction::Zoom | WindowControlAction::Expand => {
                if is_fullscreen_symbol {
                    SVG_ZOOM
                } else {
                    SVG_MAXIMIZE
                }
            }
        };

        let glyph_handle = svg::Handle::from_memory(svg_str.as_bytes());
        svg(glyph_handle)
            .width(Length::Fixed(glyph_size))
            .height(Length::Fixed(glyph_size))
            .opacity(hover)
            .style(move |_theme, _status| svg::Style {
                color: Some(glyph_color),
            })
            .into()
    };

    let shadow = if is_dark {
        iced::Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
            offset: Vector::new(0.0, 0.5),
            blur_radius: 1.5,
        }
    } else {
        iced::Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
            offset: Vector::new(0.0, 0.5),
            blur_radius: 1.0,
        }
    };

    let button_body = container(centered(glyph_widget, visual_size))
        .width(Length::Fixed(visual_size))
        .height(Length::Fixed(visual_size))
        .style(move |_theme| container::Style {
            background: Some(Background::Color(fill)),
            border: iced::Border::default()
                .rounded(visual_size * 0.5)
                .width(0.5)
                .color(border),
            shadow,
            ..Default::default()
        });

    let centered_button = centered(button_body.into(), size);
    mouse_area(centered_button)
        .on_press(on_action)
        .into()
}

/// Builds an authentic macOS traffic lights widget row.
pub fn view_traffic_lights<'a, Message: Clone + 'a, Theme: 'a + container::Catalog + svg::Catalog, Renderer>(
    config: TrafficLightsViewConfig,
    on_action: impl Fn(WindowControlAction) -> Message + 'a,
    on_group_hover: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer + advanced_text::Renderer + advanced_svg::Renderer + 'a,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    <Theme as svg::Catalog>::Class<'a>: From<svg::StyleFn<'a, Theme>>,
{
    let slop = metrics::control_hover_slop(DIAMETER);
    let hover = config.hover_progress;
    let is_active = config.is_focused || hover > 0.05;

    let close_btn = view_single_button(
        WindowControlAction::Close,
        DIAMETER,
        config.is_dark,
        is_active,
        hover,
        config.press_scales[0],
        config.is_fullscreen_symbol,
        on_action(WindowControlAction::Close),
    );

    let min_btn = view_single_button(
        WindowControlAction::Minimize,
        DIAMETER,
        config.is_dark,
        is_active,
        hover,
        config.press_scales[1],
        config.is_fullscreen_symbol,
        on_action(WindowControlAction::Minimize),
    );

    let zoom_btn = view_single_button(
        WindowControlAction::Zoom,
        DIAMETER,
        config.is_dark,
        is_active,
        hover,
        config.press_scales[2],
        config.is_fullscreen_symbol,
        on_action(WindowControlAction::Zoom),
    );

    let row_widget = row![close_btn, min_btn, zoom_btn]
        .spacing(SPACING)
        .align_y(iced::Alignment::Center);

    let area = mouse_area(row_widget)
        .on_enter(on_group_hover(true))
        .on_exit(on_group_hover(false));

    container(area)
        .padding(Padding {
            top: slop,
            right: slop,
            bottom: slop,
            left: slop,
        })
        .into()
}

/// Helper to render a group of controls with callbacks for advanced showcases.
#[allow(clippy::too_many_arguments)]
pub fn control_group<'a, Id: Copy + 'a, Message: Clone + 'a, Theme: 'a + container::Catalog + svg::Catalog, Renderer>(
    ids: [Id; 3],
    size: f32,
    gap: f32,
    is_dark: bool,
    show_glyphs: bool,
    close_disabled: bool,
    interactive: bool,
    inactive: bool,
    hover_amount: f32,
    expand_behavior: WindowExpandBehavior,
    press_scales: [f32; 3],
    on_action: impl Fn(Id, WindowControlAction) -> Message + 'a,
    on_press_start: impl Fn(Id) -> Message + 'a,
    on_press_cancel: impl Fn(Id) -> Message + 'a,
    on_press_end: impl Fn(Id) -> Message + 'a,
    on_group_hover: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer + advanced_text::Renderer + advanced_svg::Renderer + 'a,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    <Theme as svg::Catalog>::Class<'a>: From<svg::StyleFn<'a, Theme>>,
{
    let _ = (on_press_start, on_press_cancel, on_press_end);
    let hover = if show_glyphs { hover_amount } else { 0.0 };
    let is_active = !inactive;
    let is_fullscreen = expand_behavior == WindowExpandBehavior::Fullscreen;

    let btn0 = view_single_button(
        WindowControlAction::Close,
        size,
        is_dark,
        is_active && !close_disabled,
        hover,
        press_scales[0],
        is_fullscreen,
        on_action(ids[0], WindowControlAction::Close),
    );

    let btn1 = view_single_button(
        WindowControlAction::Minimize,
        size,
        is_dark,
        is_active,
        hover,
        press_scales[1],
        is_fullscreen,
        on_action(ids[1], WindowControlAction::Minimize),
    );

    let btn2 = view_single_button(
        WindowControlAction::Expand,
        size,
        is_dark,
        is_active,
        hover,
        press_scales[2],
        is_fullscreen,
        on_action(ids[2], WindowControlAction::Expand),
    );

    let controls = row![btn0, btn1, btn2].spacing(gap).align_y(iced::Alignment::Center);
    let slop = metrics::control_hover_slop(size);
    let tracking_area = container(controls).padding(Padding {
        top: slop,
        right: slop,
        bottom: slop,
        left: slop,
    });

    if interactive {
        mouse_area(tracking_area)
            .on_enter(on_group_hover(true))
            .on_exit(on_group_hover(false))
            .into()
    } else {
        tracking_area.into()
    }
}

/// Interactive version of view_traffic_lights accepting separate callbacks and state.
#[allow(clippy::too_many_arguments)]
pub fn view_traffic_lights_interactive<'a, Message: Clone + 'a, Theme: 'a + container::Catalog + svg::Catalog, Renderer>(
    state: &'a TrafficLightsState,
    is_dark: bool,
    inactive: bool,
    on_action: impl Fn(WindowControlAction) -> Message + 'a,
    on_press_start: impl Fn(usize) -> Message + 'a,
    on_press_cancel: impl Fn(usize) -> Message + 'a,
    on_press_end: impl Fn(usize) -> Message + 'a,
    on_group_hover: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer + advanced_text::Renderer + advanced_svg::Renderer + 'a,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    <Theme as svg::Catalog>::Class<'a>: From<svg::StyleFn<'a, Theme>>,
{
    let press_scales = [
        state.press_springs[0].value(),
        state.press_springs[1].value(),
        state.press_springs[2].value(),
    ];
    control_group(
        WINDOW_CONTROL_NATIVE_IDS,
        WINDOW_CONTROL_NATIVE_SIZE,
        WINDOW_CONTROL_GAP,
        is_dark,
        true,
        false,
        true,
        inactive,
        state.hover_progress,
        state.expand_behavior,
        press_scales,
        move |_id, action| on_action(action),
        on_press_start,
        on_press_cancel,
        on_press_end,
        on_group_hover,
    )
}

/// Helper to position a control group with optical slop compensation.
#[must_use]
pub fn positioned_control_group<'a, Message: 'a, Theme: 'a + container::Catalog, Renderer: advanced::Renderer + 'a>(
    content: Element<'a, Message, Theme, Renderer>,
    x: f32,
    y: f32,
    size: f32,
) -> Element<'a, Message, Theme, Renderer>
where
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
{
    let slop = control_hover_slop(size);
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding {
            top: y - slop,
            right: 0.0,
            bottom: 0.0,
            left: x - slop,
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traffic_lights_metrics_consistency() {
        assert_eq!(DIAMETER, 14.0);
        assert_eq!(SPACING, 9.0);
        assert_eq!(metrics::TOTAL_WIDTH, 60.0);
        assert_eq!(WINDOW_CONTROL_NATIVE_SIZE, 14.0);
        assert_eq!(WINDOW_CONTROL_GAP, 9.0);
    }

    #[test]
    fn test_traffic_lights_state_spring_dynamics() {
        let mut state = TrafficLightsState::new();
        assert!(!state.is_animating());

        state.on_group_hover(true);
        assert_eq!(state.hover_target, 1.0);

        state.on_press_start(0);
        assert!(state.is_animating());
        assert_eq!(state.press_targets[0], 1.0);

        state.step(Instant::now());
        assert!(state.hover_progress > 0.0);

        state.on_press_end(0);
        assert_eq!(state.press_targets[0], 0.0);
    }

    #[test]
    fn test_traffic_lights_view_config_builder() {
        let mut state = TrafficLightsState::new();
        state.hover_progress = 0.8;
        let config = TrafficLightsViewConfig::from_state(&state, true, true);

        assert!(config.is_focused);
        assert!(config.is_dark);
        assert_eq!(config.hover_progress, 0.8);
        assert_eq!(config.press_scales[0], 1.0);
    }

    #[test]
    fn test_traffic_lights_inactive_distinguishes_dark_and_light() {
        let (dark_fill, dark_border) = resolve_inactive_button_colors(true);
        let (light_fill, light_border) = resolve_inactive_button_colors(false);

        assert_eq!(dark_fill, Color::from_rgb8(0x4C, 0x4C, 0x50));
        assert_eq!(light_fill, Color::from_rgb8(0xD1, 0xD1, 0xD6));
        assert!(light_fill.r > dark_fill.r);
        assert_ne!(dark_border, light_border);
    }

    #[test]
    fn test_traffic_lights_dark_mode_active_borders() {
        let (_dark_fill, dark_border) =
            resolve_active_button_colors(WindowControlAction::Close, true, false, false);
        let (_light_fill, light_border) =
            resolve_active_button_colors(WindowControlAction::Close, false, false, false);

        assert_eq!(dark_border, Color::from_rgb8(0xB8, 0x32, 0x2B));
        assert_eq!(light_border, Color::from_rgb8(0xE0, 0x44, 0x3E));
        assert!(light_border.r > dark_border.r);
    }

    #[test]
    fn test_traffic_lights_glyph_color_transparency() {
        let no_hover = window_control_glyph_color(false, WindowControlAction::Close, false, 0.0);
        let full_hover_light =
            window_control_glyph_color(false, WindowControlAction::Close, false, 1.0);
        let full_hover_dark =
            window_control_glyph_color(true, WindowControlAction::Close, false, 1.0);

        assert_eq!(no_hover.a, 0.0);
        assert_eq!(full_hover_light.a, 0.75);
        assert_eq!(full_hover_dark.a, 0.85);
    }
}
