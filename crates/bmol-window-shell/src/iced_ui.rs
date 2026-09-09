//! Iced UI integrations for bmol-window-shell.
//!
//! Provides the official outer rim wrapper (`wrap_window_rim`) and loyal
//! drag bar (`loyal_drag_bar`) adhering to macOS and Linux design invariants.

pub mod controller;
pub mod resizer;
pub mod traffic_lights;

pub use controller::{ShellEvent, WindowShellController};
pub use resizer::{
    resize_direction_to_interaction, resize_direction_to_window_direction, resolve_resize_at,
    wrap_border_resizer,
};
pub use traffic_lights::{
    ControlAction, TrafficLightButton, TrafficLightsEvent, TrafficLightsState,
    WindowControlAction, is_document_edited, glass_passthrough, set_document_edited, set_glass_passthrough,
    view_single_button, view_single_button_interactive, view_traffic_lights_all_inclusive,
};

use iced::{
    Color, Element, Length,
    widget::{container, mouse_area},
};

use crate::platform::{WindowState, window_rim};

/// Configuration for the non-client window rim container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowRimConfig {
    /// Whether dark mode rim styling is active.
    pub is_dark: bool,
    /// Outer corner radius of the window rim.
    pub corner_radius: f32,
    /// Window state (Normal, Maximized, Fullscreen).
    pub state: WindowState,
}

impl WindowRimConfig {
    /// Creates a default rim configuration for the given color scheme.
    #[must_use]
    pub const fn new(is_dark: bool) -> Self {
        Self {
            is_dark,
            corner_radius: 10.0,
            state: WindowState::Normal,
        }
    }

    /// Sets the outer corner radius.
    #[must_use]
    pub const fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Sets the window state.
    #[must_use]
    pub const fn with_state(mut self, state: WindowState) -> Self {
        self.state = state;
        self
    }
}

impl From<bool> for WindowRimConfig {
    fn from(is_dark: bool) -> Self {
        Self::new(is_dark)
    }
}

impl From<(bool, f32)> for WindowRimConfig {
    fn from((is_dark, corner_radius): (bool, f32)) -> Self {
        Self::new(is_dark).with_corner_radius(corner_radius)
    }
}

/// Wraps client content with authentic macOS / Linux window outer rim borders.
///
/// Accepts either a [`WindowRimConfig`], a `bool` (for `is_dark`), or a tuple `(is_dark, corner_radius)`.
///
/// Under Dark Mode:
/// - Outer layer: 1px deep delineation line (`rgba(0, 0, 0, 0.85)`) with 1.0 padding.
/// - Inner layer: 1px highlight bevel line (`rgb(70, 70, 70)`) with 1.0 padding.
///
/// Under Light Mode:
/// - Single layer: 1px subtle line (`rgba(0, 0, 0, 0.10)`) with 1.0 padding.
///
/// Under Fullscreen:
/// - Insets automatically collapse to zero (no padding or borders added).
pub fn wrap_window_rim<'a, Message: 'a, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    config: impl Into<WindowRimConfig>,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: 'a + iced::widget::container::Catalog,
    Theme::Class<'a>: From<iced::widget::container::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::Renderer + 'a,
{
    let config = config.into();

    // When in fullscreen mode, rims collapse completely to zero
    if config.state == WindowState::Fullscreen {
        return content.into();
    }

    let outer_radius = config.corner_radius;

    if config.is_dark {
        let (r, g, b) = window_rim::DARK_INNER_RIM_COLOR_RGB8;
        let inner_color = Color::from_rgb8(r, g, b);
        let (outer_r, outer_g, outer_b, outer_a) = window_rim::DARK_OUTER_RIM_COLOR_RGBA;
        let outer_color = Color::from_rgba(outer_r, outer_g, outer_b, outer_a);

        let inner_window = container(content.into())
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(1.0)
            .style(move |_theme| container::Style {
                border: iced::Border {
                    color: inner_color,
                    width: 1.0,
                    radius: (outer_radius - 1.0).max(0.0).into(),
                },
                ..Default::default()
            });

        container(inner_window)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(1.0)
            .style(move |_theme| container::Style {
                border: iced::Border {
                    color: outer_color,
                    width: 1.0,
                    radius: outer_radius.into(),
                },
                ..Default::default()
            })
            .into()
    } else {
        let (lr, lg, lb, la) = window_rim::LIGHT_RIM_COLOR_RGBA;
        let light_color = Color::from_rgba(lr, lg, lb, la);

        container(content.into())
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(1.0)
            .style(move |_theme| container::Style {
                border: iced::Border {
                    color: light_color,
                    width: 1.0,
                    radius: outer_radius.into(),
                },
                ..Default::default()
            })
            .into()
    }
}

/// A transparent, faithful draggable header bar for custom window chromes.
///
/// Interactive child widgets consume their own clicks and gestures, while
/// clicking or dragging empty areas or backgrounds triggers window dragging.
pub fn loyal_drag_bar<'a, Message: 'a + Clone, Theme, Renderer>(
    height: f32,
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    on_drag: Message,
    on_double_click: Option<Message>,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: 'a + iced::widget::container::Catalog,
    Theme::Class<'a>: From<iced::widget::container::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::Renderer + 'a,
{
    let mut area = mouse_area(
        container(content.into())
            .width(Length::Fill)
            .height(Length::Fixed(height))
            .style(|_theme| container::Style {
                background: None,
                ..Default::default()
            }),
    )
    .on_press(on_drag);

    if let Some(double_click_msg) = on_double_click {
        area = area.on_double_click(double_click_msg);
    }

    area.into()
}

/// Streams the operating system light/dark appearance.
///
/// iced already listens to the platform, so the window shell simply maps that
/// `Mode` stream into the application's message. This is the single supported
/// way for a shell app to follow the system theme.
#[cfg(feature = "theme")]
pub fn system_theme_subscription<Message: 'static>(
    f: impl Fn(iced::theme::Mode) -> Message + 'static + Send + Sync + Clone,
) -> iced::Subscription<Message> {
    iced::system::theme_changes().map(f)
}

/// Returns the initial OS dark-mode state synchronously, for boot state.
///
/// Use [`system_theme_subscription`] for subsequent changes so the shell and
/// the app never disagree about the current appearance.
#[cfg(feature = "theme")]
#[must_use]
pub fn initial_system_dark_mode() -> bool {
    crate::native::is_system_dark_mode()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_rim_config() {
        let config = WindowRimConfig::new(true)
            .with_corner_radius(12.0)
            .with_state(WindowState::Maximized);

        assert!(config.is_dark);
        assert_eq!(config.corner_radius, 12.0);
        assert_eq!(config.state, WindowState::Maximized);

        let from_bool: WindowRimConfig = false.into();
        assert!(!from_bool.is_dark);
        assert_eq!(from_bool.corner_radius, 10.0);

        let from_tuple: WindowRimConfig = (true, 16.0).into();
        assert!(from_tuple.is_dark);
        assert_eq!(from_tuple.corner_radius, 16.0);
    }
}


