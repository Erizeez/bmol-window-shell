//! Native Apple-grade Traffic Lights window controls (Close, Minimize, Zoom).
//!
//! Delivers pixel-perfect macOS traffic light buttons (14px diameter, 9px gap)
//! with dual-tone edge borders, group hover symbol reveals, active/inactive
//! window substrate blending, and crisp vector glyphs without external assets.

use iced::advanced::text as advanced_text;
use iced::advanced::{self};
use iced::widget::{container, mouse_area, row, space, text};
use iced::{Alignment, Color, Element, Length};

use crate::platform::traffic_lights::SPACING;

/// Native diameter of a macOS traffic light button (14.0 pt).
pub const NATIVE_TRAFFIC_LIGHT_SIZE: f32 = 14.0;
/// Authentic gap between traffic light buttons (strictly 9.0 pt).
pub const NATIVE_TRAFFIC_LIGHT_GAP: f32 = SPACING;

/// Actions emitted by the traffic lights control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficLightsAction {
    Close,
    Minimize,
    Zoom,
}

/// Visual configuration for the traffic lights.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrafficLightsConfig {
    /// Whether the window is in dark appearance mode.
    pub is_dark: bool,
    /// Whether the window currently has active keyboard/app focus.
    pub is_focused: bool,
    /// Button diameter in logical points (defaults to standard 14.0).
    pub size: f32,
    /// Net gap between buttons (defaults to standard 9.0).
    pub gap: f32,
}

impl Default for TrafficLightsConfig {
    fn default() -> Self {
        Self {
            is_dark: false,
            is_focused: true,
            size: NATIVE_TRAFFIC_LIGHT_SIZE,
            gap: NATIVE_TRAFFIC_LIGHT_GAP,
        }
    }
}

impl TrafficLightsConfig {
    #[must_use]
    pub const fn new(is_dark: bool, is_focused: bool) -> Self {
        Self {
            is_dark,
            is_focused,
            size: NATIVE_TRAFFIC_LIGHT_SIZE,
            gap: NATIVE_TRAFFIC_LIGHT_GAP,
        }
    }

    #[must_use]
    pub const fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub const fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Total width required for the 3 buttons and 2 gaps (e.g. 14*3 + 9*2 = 60.0).
    #[must_use]
    pub fn total_width(&self) -> f32 {
        self.size * 3.0 + self.gap * 2.0
    }
}

/// Persistent interaction state for the traffic lights control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrafficLightsState {
    /// Whether the mouse cursor is currently hovering within the control group.
    pub is_hovered: bool,
}

impl TrafficLightsState {
    #[must_use]
    pub const fn new() -> Self {
        Self { is_hovered: false }
    }

    pub fn set_hovered(&mut self, hovered: bool) {
        self.is_hovered = hovered;
    }
}

/// Resolves fill and 1px border stroke colors for an action.
#[must_use]
pub fn resolve_button_colors(
    action: TrafficLightsAction,
    config: &TrafficLightsConfig,
) -> (Color, Color) {
    if !config.is_focused {
        if config.is_dark {
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
            TrafficLightsAction::Close => {
                (Color::from_rgb8(0xFF, 0x5F, 0x56), Color::from_rgb8(0xE0, 0x44, 0x3E))
            }
            TrafficLightsAction::Minimize => {
                (Color::from_rgb8(0xFF, 0xBD, 0x2E), Color::from_rgb8(0xDE, 0xA1, 0x23))
            }
            TrafficLightsAction::Zoom => {
                (Color::from_rgb8(0x27, 0xC9, 0x3F), Color::from_rgb8(0x1A, 0xAB, 0x29))
            }
        }
    }
}

/// Resolves the foreground glyph color when hovered.
#[must_use]
pub fn resolve_glyph_color(action: TrafficLightsAction, config: &TrafficLightsConfig) -> Color {
    if !config.is_focused {
        if config.is_dark {
            Color::from_rgba(0.82, 0.83, 0.86, 0.76)
        } else {
            Color::from_rgba(0.52, 0.53, 0.56, 0.78)
        }
    } else {
        match action {
            TrafficLightsAction::Close => Color::from_rgba(0.38, 0.10, 0.08, 0.92),
            TrafficLightsAction::Minimize => Color::from_rgba(0.42, 0.28, 0.03, 0.92),
            TrafficLightsAction::Zoom => Color::from_rgba(0.09, 0.32, 0.07, 0.92),
        }
    }
}

/// Renders a single traffic light button with authentic colors and centered glyph.
pub fn view_single_button<'a, Message: 'a + Clone, Theme, Renderer>(
    action: TrafficLightsAction,
    config: &TrafficLightsConfig,
    is_hovered: bool,
    on_action: Message,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: 'a + container::Catalog + iced::widget::text::Catalog,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    <Theme as iced::widget::text::Catalog>::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + advanced_text::Renderer + 'a,
{
    let size = config.size;
    let radius = size * 0.5;
    let (fill, border) = resolve_button_colors(action, config);

    let symbol_char = match action {
        TrafficLightsAction::Close => "✕",
        TrafficLightsAction::Minimize => "−",
        TrafficLightsAction::Zoom => "⤢",
    };

    let symbol_color = if is_hovered && config.is_focused {
        resolve_glyph_color(action, config)
    } else {
        Color::TRANSPARENT
    };

    let symbol = text(symbol_char)
        .size((size * 0.55).max(7.0))
        .color(symbol_color);

    let btn_content = container(symbol)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .style(move |_theme| container::Style {
            background: Some(fill.into()),
            border: iced::Border {
                color: border,
                width: 1.0,
                radius: radius.into(),
            },
            ..Default::default()
        });

    mouse_area(btn_content).on_press(on_action).into()
}

/// Renders the complete traffic lights control group with group-hover sensing.
pub fn view_traffic_lights<'a, Message: 'a + Clone, Theme, Renderer>(
    config: TrafficLightsConfig,
    state: &'a TrafficLightsState,
    on_action: impl Fn(TrafficLightsAction) -> Message + 'a,
    on_group_hover: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: 'a + container::Catalog + iced::widget::text::Catalog,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    <Theme as iced::widget::text::Catalog>::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + advanced_text::Renderer + 'a,
{
    let close = view_single_button(
        TrafficLightsAction::Close,
        &config,
        state.is_hovered,
        on_action(TrafficLightsAction::Close),
    );

    let minimize = view_single_button(
        TrafficLightsAction::Minimize,
        &config,
        state.is_hovered,
        on_action(TrafficLightsAction::Minimize),
    );

    let zoom = view_single_button(
        TrafficLightsAction::Zoom,
        &config,
        state.is_hovered,
        on_action(TrafficLightsAction::Zoom),
    );

    let gap_spacer1 = space().width(Length::Fixed(config.gap));
    let gap_spacer2 = space().width(Length::Fixed(config.gap));

    let buttons_row = row![close, gap_spacer1, minimize, gap_spacer2, zoom]
        .align_y(Alignment::Center)
        .width(Length::Fixed(config.total_width()))
        .height(Length::Fixed(config.size));

    mouse_area(buttons_row)
        .on_enter(on_group_hover(true))
        .on_exit(on_group_hover(false))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traffic_lights_specifications() {
        let config = TrafficLightsConfig::default();
        assert_eq!(config.size, 14.0);
        assert_eq!(config.gap, 9.0);
        assert_eq!(config.total_width(), 14.0 * 3.0 + 9.0 * 2.0);
        assert_eq!(config.total_width(), 60.0);
    }

    #[test]
    fn test_button_colors_active_vs_inactive() {
        let active = TrafficLightsConfig::new(true, true);
        let inactive = TrafficLightsConfig::new(true, false);

        let (act_fill, act_border) = resolve_button_colors(TrafficLightsAction::Close, &active);
        let (inact_fill, inact_border) =
            resolve_button_colors(TrafficLightsAction::Close, &inactive);

        assert_ne!(act_fill, inact_fill);
        assert_ne!(act_border, inact_border);
    }
}
