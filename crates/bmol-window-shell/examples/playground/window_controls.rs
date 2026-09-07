//! Reusable macOS Liquid Glass Window Controls (Traffic Lights).
//!
//! Provides pixel-accurate macOS window controls (Close, Minimize, Zoom/Expand)
//! with liquid-glass styling, spring physics scaling, extracted Apple vector glyphs,
//! group hover sensing, and inactive window state blending.

use std::time::Instant;

use iced::advanced::{self, svg as advanced_svg};
use iced::widget::{button, container, mouse_area, row, space};
use iced::{Alignment, Background, Color, Element, Length, Padding, Theme};
use liquid_glass::{UiColorScheme, UiIcon, WindowExpandBehavior};
use liquid_glass_ui::components;
use spring_rs::{Spring, SpringMotion};

// Standard macOS traffic light dimensions
pub const WINDOW_CONTROL_NATIVE_SIZE: f32 = 14.0;
pub const WINDOW_CONTROL_LARGE_SIZE: f32 = 64.0;
pub const WINDOW_CONTROL_GAP: f32 = 7.0;
pub const WINDOW_CONTROL_LARGE_GAP: f32 = 12.0;

// Spring physics constants for bouncy clicks
pub const PRESS_SCALE_OVERSHOOT: f32 = 1.18;
pub const PRESS_SCALE_SETTLED: f32 = 1.06;
pub const PRESS_SCALE_SPRING_DURATION: f32 = 0.24;
pub const PRESS_SCALE_SPRING_EXTRA_BOUNCE: f32 = 0.40;
pub const INTERACTION_ENTER_ANIMATION_TIME_CONSTANT: f32 = 0.08;
pub const INTERACTION_EXIT_ANIMATION_TIME_CONSTANT: f32 = 0.18;

/// A window control semantic action.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ControlAction {
    Close,
    Minimize,
    Expand,
}

impl ControlAction {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Close => "Close",
            Self::Minimize => "Minimize",
            Self::Expand => "Expand",
        }
    }
}

/// Control group variants for preview and inspection.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ControlGroup {
    Native,
    Reference,
    Active,
    Inactive,
    Disabled,
}

impl ControlGroup {
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

/// Dynamic state managing the liquid glass traffic lights (springs, hover fade, tick timer).
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
            now.saturating_duration_since(last).as_secs_f32().min(0.1)
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

    pub fn is_animating(&self) -> bool {
        (self.hover_target - self.hover_progress).abs() > 0.001
            || self.press_springs.iter().any(|s| !s.is_settled(0.001, 0.01))
    }
}

/// Measured glyph size inside a control circle of `size`.
pub fn window_control_glyph_size(action: ControlAction, size: f32) -> f32 {
    let factor = if size <= WINDOW_CONTROL_NATIVE_SIZE {
        match action {
            ControlAction::Close => 7.0 / 14.0,
            ControlAction::Minimize => 8.0 / 14.0,
            ControlAction::Expand => 0.42,
        }
    } else {
        0.42
    };
    (size * factor).max(4.0)
}

/// Color for the control glyph based on action, color scheme, inactive state, and focus amount.
pub fn window_control_glyph_color(
    scheme: UiColorScheme,
    action: ControlAction,
    inactive: bool,
    focus_amount: f32,
) -> Color {
    let focus_amount = focus_amount.clamp(0.0, 1.0);
    let active = match action {
        ControlAction::Close => Color::from_rgba(0.38, 0.10, 0.08, 0.92),
        ControlAction::Minimize => Color::from_rgba(0.42, 0.28, 0.03, 0.92),
        ControlAction::Expand => Color::from_rgba(0.09, 0.32, 0.07, 0.92),
    };
    let inactive_color = match scheme {
        UiColorScheme::Light => Color::from_rgba(0.52, 0.53, 0.56, 0.78),
        UiColorScheme::Dark => Color::from_rgba(0.82, 0.83, 0.86, 0.76),
    };
    let color = if inactive { blend_color(inactive_color, active, focus_amount) } else { active };
    Color::from_rgba(color.r, color.g, color.b, color.a * focus_amount)
}

/// Returns the matching vector icon for `action`.
pub fn window_control_icon(action: ControlAction, expand_behavior: WindowExpandBehavior) -> UiIcon {
    match action {
        ControlAction::Close => UiIcon::WindowClose,
        ControlAction::Minimize => UiIcon::WindowMinimize,
        ControlAction::Expand => expand_behavior.icon(),
    }
}

/// Expanded hover detection buffer around window controls to prevent flicker.
pub fn control_hover_slop(size: f32) -> f32 {
    (size * 0.20).clamp(6.0, 12.0)
}

/// Blends two colors linearly by `amount`.
pub fn blend_color(from: Color, to: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color::from_rgba(
        from.r + (to.r - from.r) * amount,
        from.g + (to.g - from.g) * amount,
        from.b + (to.b - from.b) * amount,
        from.a + (to.a - from.a) * amount,
    )
}

/// Status dot for disabled or unsaved close control.
pub fn window_control_status_dot<'a, Message: 'a, Renderer>(
    color: Color,
    size: f32,
) -> Element<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer + 'a,
{
    let dot_size = (size * 0.24).max(3.0);
    container(space())
        .width(Length::Fixed(dot_size))
        .height(Length::Fixed(dot_size))
        .style(move |_theme| container::Style {
            background: Some(Background::Color(color)),
            border: iced::Border::default().rounded(dot_size * 0.5),
            ..container::Style::default()
        })
        .into()
}

/// Centered container helper for optical alignment.
pub fn centered<'a, Message: 'a, Renderer>(
    content: Element<'a, Message, Theme, Renderer>,
    size: f32,
) -> Element<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer + 'a,
{
    container(content)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .into()
}

/// Authentic liquid-glass colors and borders for traffic light buttons.
pub fn traffic_light_appearance(
    action: ControlAction,
    is_active: bool,
    is_dark: bool,
    is_hover: bool,
    is_press: bool,
) -> (Color, Color) {
    if !is_active {
        if is_dark {
            (
                Color::from_rgba(0.32, 0.32, 0.35, 0.50),
                Color::from_rgba(0.24, 0.24, 0.26, 0.40),
            )
        } else {
            (
                Color::from_rgba(0.85, 0.85, 0.87, 0.95),
                Color::from_rgba(0.72, 0.72, 0.75, 0.70),
            )
        }
    } else {
        match action {
            ControlAction::Close => {
                if is_press {
                    (Color::from_rgb8(0xD2, 0x3C, 0x34), Color::from_rgb8(0xBF, 0x34, 0x2D))
                } else if is_hover {
                    (Color::from_rgb8(0xFF, 0x6E, 0x67), Color::from_rgb8(0xE2, 0x4B, 0x46))
                } else {
                    (Color::from_rgb8(0xFF, 0x5F, 0x56), Color::from_rgb8(0xE0, 0x44, 0x3E))
                }
            }
            ControlAction::Minimize => {
                if is_press {
                    (Color::from_rgb8(0xD7, 0x96, 0x1E), Color::from_rgb8(0xC2, 0x82, 0x16))
                } else if is_hover {
                    (Color::from_rgb8(0xFF, 0xC8, 0x47), Color::from_rgb8(0xDF, 0xA9, 0x32))
                } else {
                    (Color::from_rgb8(0xFF, 0xBD, 0x2E), Color::from_rgb8(0xDE, 0xA1, 0x23))
                }
            }
            ControlAction::Expand => {
                if is_press {
                    (Color::from_rgb8(0x19, 0xA0, 0x23), Color::from_rgb8(0x14, 0x8C, 0x1C))
                } else if is_hover {
                    (Color::from_rgb8(0x32, 0xD8, 0x4D), Color::from_rgb8(0x23, 0xB6, 0x33))
                } else {
                    (Color::from_rgb8(0x27, 0xC9, 0x3F), Color::from_rgb8(0x1A, 0xAB, 0x29))
                }
            }
        }
    }
}

/// Builds an individual Liquid Glass traffic light button.
pub fn view_window_control<'a, Message: 'a, Renderer>(
    action: ControlAction,
    size: f32,
    scale: f32,
    scheme: UiColorScheme,
    inactive: bool,
    hover_amount: f32,
    expand_behavior: WindowExpandBehavior,
    on_action: Message,
    on_press_start: Message,
    on_press_end: Message,
) -> Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: advanced::Renderer + advanced_svg::Renderer + 'a,
{
    let visual_size = size * scale;
    let is_dark = matches!(scheme, UiColorScheme::Dark);
    let is_active = !inactive || hover_amount > 0.05;

    let glyph_color = window_control_glyph_color(scheme, action, inactive, hover_amount);
    let glyph_size = window_control_glyph_size(action, size) * scale;
    let glyph = components::icon_tinted_with_opacity(
        window_control_icon(action, expand_behavior),
        glyph_size,
        glyph_color,
    );

    let content: Element<'a, Message, Theme, Renderer> = centered(glyph, visual_size);

    let btn: Element<'a, Message, Theme, Renderer> = button(content)
        .padding(0)
        .width(Length::Fixed(visual_size))
        .height(Length::Fixed(visual_size))
        .on_press(on_action)
        .style(move |_theme: &Theme, status| {
            let is_hover = matches!(status, button::Status::Hovered);
            let is_press = matches!(status, button::Status::Pressed);
            let (fill, stroke) = traffic_light_appearance(action, is_active, is_dark, is_hover, is_press);

            button::Style {
                background: Some(fill.into()),
                border: iced::Border {
                    color: stroke,
                    width: 0.5,
                    radius: (visual_size * 0.5).into(),
                },
                ..Default::default()
            }
        })
        .into();

    let btn_centered: Element<'a, Message, Theme, Renderer> = centered(btn, size);

    mouse_area(btn_centered)
        .on_press(on_press_start)
        .on_release(on_press_end)
        .into()
}

/// Renders the complete, authentic Apple Liquid Glass traffic lights row.
pub fn view_traffic_lights<'a, Message: 'a, Renderer>(
    state: &'a TrafficLightsState,
    scheme: UiColorScheme,
    inactive: bool,
    on_action: impl Fn(ControlAction) -> Message + Copy + 'a,
    on_press_start: impl Fn(usize) -> Message + Copy + 'a,
    on_press_end: impl Fn(usize) -> Message + Copy + 'a,
    on_group_hover: impl Fn(bool) -> Message + Copy + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: advanced::Renderer + advanced_svg::Renderer + 'a,
{
    let size = WINDOW_CONTROL_NATIVE_SIZE;
    let gap = WINDOW_CONTROL_GAP;

    let close_btn = view_window_control(
        ControlAction::Close,
        size,
        state.press_springs[0].value(),
        scheme,
        inactive,
        state.hover_progress,
        state.expand_behavior,
        on_action(ControlAction::Close),
        on_press_start(0),
        on_press_end(0),
    );

    let min_btn = view_window_control(
        ControlAction::Minimize,
        size,
        state.press_springs[1].value(),
        scheme,
        inactive,
        state.hover_progress,
        state.expand_behavior,
        on_action(ControlAction::Minimize),
        on_press_start(1),
        on_press_end(1),
    );

    let zoom_btn = view_window_control(
        ControlAction::Expand,
        size,
        state.press_springs[2].value(),
        scheme,
        inactive,
        state.hover_progress,
        state.expand_behavior,
        on_action(ControlAction::Expand),
        on_press_start(2),
        on_press_end(2),
    );

    let controls = row![close_btn, min_btn, zoom_btn].spacing(gap).align_y(Alignment::Center);

    let slop = control_hover_slop(size);
    let group_width = size * 3.0 + gap * 2.0;

    let tracking_area = container(controls)
        .width(Length::Fixed(group_width + slop * 2.0))
        .height(Length::Fixed(size + slop * 2.0))
        .padding(Padding { top: slop, right: slop, bottom: slop, left: slop });

    mouse_area(tracking_area)
        .on_enter(on_group_hover(true))
        .on_exit(on_group_hover(false))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sizes_and_gaps() {
        assert_eq!(WINDOW_CONTROL_NATIVE_SIZE, 14.0);
        assert_eq!(WINDOW_CONTROL_GAP, 7.0);
        assert_eq!(WINDOW_CONTROL_LARGE_SIZE, 64.0);
    }

    #[test]
    fn test_glyph_sizes() {
        let close = window_control_glyph_size(ControlAction::Close, WINDOW_CONTROL_NATIVE_SIZE);
        let minimize = window_control_glyph_size(ControlAction::Minimize, WINDOW_CONTROL_NATIVE_SIZE);
        let expand = window_control_glyph_size(ControlAction::Expand, WINDOW_CONTROL_NATIVE_SIZE);

        assert_eq!(close, 7.0);
        assert_eq!(minimize, 8.0);
        assert!(minimize > close);
        assert!(close > expand);
    }

    #[test]
    fn test_glyph_colors() {
        let active_light =
            window_control_glyph_color(UiColorScheme::Light, ControlAction::Close, false, 1.0);
        let inactive_light =
            window_control_glyph_color(UiColorScheme::Light, ControlAction::Close, true, 0.0);

        assert!(inactive_light.r > active_light.r);
        assert_eq!(inactive_light.a, 0.0);
    }

    #[test]
    fn test_spring_dynamics() {
        let mut state = TrafficLightsState::new();
        assert!(!state.is_animating());

        state.on_press_start(0);
        assert!(state.is_animating());
        assert_eq!(state.press_targets[0], 1.0);
    }
}
