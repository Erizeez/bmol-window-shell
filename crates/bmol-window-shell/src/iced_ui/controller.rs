//! Application-level window shell state machine and runtime controller.
//!
//! Encapsulates metrics computation, display scale tracking, window state
//! transitions, and seamless wrapping for downstream applications.

use iced::Element;
use iced::window;

use crate::platform::{
    ChromeLayoutMode, ResizeDirection, WindowChromeConfig, WindowChromeMetrics, WindowState,
};

use super::resizer::resolve_resize_at;
use super::traffic_lights::{
    TrafficLightsAction, TrafficLightsConfig, TrafficLightsState, view_traffic_lights,
};
use super::{WindowRimConfig, loyal_drag_bar, wrap_window_rim};

/// High-level semantic events detected by the shell controller.
#[derive(Debug, Clone, PartialEq)]
pub enum ShellEvent {
    Resized { width: f32, height: f32 },
    ScaleFactorChanged { scale_factor: f32 },
    Focused,
    Unfocused,
    CloseRequested,
    ThemeChanged { is_dark: bool },
    WindowStateChanged(WindowState),
}

/// Persistent controller managing a window's chrome, metrics, and event loop integration.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowShellController {
    pub window_id: Option<window::Id>,
    pub window_size: (f32, f32),
    pub scale_factor: f32,
    pub state: WindowState,
    pub is_dark: bool,
    pub is_focused: bool,
    pub config: WindowChromeConfig,
    pub metrics: WindowChromeMetrics,
}

impl WindowShellController {
    /// Creates a new `WindowShellController` with the specified chrome configuration.
    #[must_use]
    pub fn new(config: WindowChromeConfig, is_dark: bool) -> Self {
        let default_width = 900.0_f32;
        let default_height = 600.0_f32;
        let scale_factor = 1.0_f32;
        let state = config.state;

        let active_config = config
            .with_state(state)
            .with_scale_factor(scale_factor);

        let metrics = WindowChromeMetrics::compute_with_theme(
            default_width,
            default_height,
            &active_config,
            is_dark,
        );

        Self {
            window_id: None,
            window_size: (default_width, default_height),
            scale_factor,
            state,
            is_dark,
            is_focused: true,
            config: active_config,
            metrics,
        }
    }

    /// Associates an Iced window ID with this controller.
    pub fn set_window_id(&mut self, window_id: window::Id) {
        self.window_id = Some(window_id);
    }

    /// Sets the layout mode (Separate or Unified) and recomputes metrics.
    pub fn set_layout_mode(&mut self, mode: ChromeLayoutMode) {
        self.config.mode = mode;
        self.recompute_metrics();
    }

    /// Sets the color appearance mode (dark vs light) and recomputes metrics.
    pub fn set_dark_mode(&mut self, is_dark: bool) {
        if self.is_dark != is_dark {
            self.is_dark = is_dark;
            self.recompute_metrics();
        }
    }

    /// Sets the window state (Normal, Maximized, Fullscreen) and recomputes metrics.
    pub fn set_window_state(&mut self, state: WindowState) {
        if self.state != state {
            self.state = state;
            self.config.state = state;
            self.recompute_metrics();
        }
    }

    /// Manually updates the window dimensions and recomputes metrics.
    pub fn set_window_size(&mut self, width: f32, height: f32) {
        if (self.window_size.0 - width).abs() > 0.001 || (self.window_size.1 - height).abs() > 0.001
        {
            self.window_size = (width.max(1.0), height.max(1.0));
            self.recompute_metrics();
        }
    }

    /// Updates the display scale factor and recomputes metrics.
    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        if (self.scale_factor - scale_factor).abs() > 0.001 {
            self.scale_factor = scale_factor.max(0.1);
            self.config.scale_factor = self.scale_factor;
            self.recompute_metrics();
        }
    }

    /// Recomputes window chrome metrics using current internal state.
    pub fn recompute_metrics(&mut self) {
        self.metrics = WindowChromeMetrics::compute_with_theme(
            self.window_size.0,
            self.window_size.1,
            &self.config,
            self.is_dark,
        );
    }

    /// Processes an Iced window event, automatically updating internal state and metrics.
    ///
    /// Returns the semantic [`ShellEvent`] if the event modified window parameters.
    pub fn handle_window_event(&mut self, event: &window::Event) -> Option<ShellEvent> {
        match event {
            window::Event::Focused => {
                self.is_focused = true;
                Some(ShellEvent::Focused)
            }
            window::Event::Unfocused => {
                self.is_focused = false;
                Some(ShellEvent::Unfocused)
            }
            window::Event::CloseRequested => Some(ShellEvent::CloseRequested),
            _ => None,
        }
    }

    /// Handles a window resized event from `window::resize_events()` or similar.
    pub fn handle_resized(&mut self, width: f32, height: f32) -> ShellEvent {
        self.set_window_size(width, height);
        ShellEvent::Resized {
            width: self.window_size.0,
            height: self.window_size.1,
        }
    }

    /// Wraps the application root element in non-client rims with safe client boundaries.
    pub fn wrap_window<'a, Message: 'a, Theme, Renderer>(
        &self,
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        corner_radius: f32,
    ) -> Element<'a, Message, Theme, Renderer>
    where
        Theme: 'a + iced::widget::container::Catalog,
        Theme::Class<'a>: From<iced::widget::container::StyleFn<'a, Theme>>,
        Renderer: iced::advanced::Renderer + 'a,
    {
        wrap_window_rim(
            content,
            WindowRimConfig::new(self.is_dark)
                .with_corner_radius(corner_radius)
                .with_state(self.state),
        )
    }

    /// Wraps the application root element in non-client rims plus interactive border resize handles.
    pub fn wrap_window_with_resizer<'a, Message: 'a + Clone, Theme, Renderer>(
        &self,
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        corner_radius: f32,
        on_resize: impl Fn(window::Direction) -> Message + 'a + Copy,
    ) -> Element<'a, Message, Theme, Renderer>
    where
        Theme: 'a + iced::widget::container::Catalog,
        Theme::Class<'a>: From<iced::widget::container::StyleFn<'a, Theme>>,
        Renderer: iced::advanced::Renderer + 'a,
    {
        let wrapped = self.wrap_window(content, corner_radius);
        super::resizer::wrap_border_resizer(
            wrapped,
            self.state == WindowState::Fullscreen,
            on_resize,
        )
    }

    /// Creates a faithful draggable header bar matching this window's chrome.
    pub fn loyal_drag_bar<'a, Message: 'a + Clone, Theme, Renderer>(
        &self,
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
        loyal_drag_bar(height, content, on_drag, on_double_click)
    }

    /// Renders standard macOS traffic lights using this controller's appearance and focus state.
    pub fn view_traffic_lights<'a, Message: 'a + Clone, Theme, Renderer>(
        &self,
        state: &'a TrafficLightsState,
        on_action: impl Fn(TrafficLightsAction) -> Message + 'a,
        on_group_hover: impl Fn(bool) -> Message + 'a,
    ) -> Element<'a, Message, Theme, Renderer>
    where
        Theme: 'a + iced::widget::container::Catalog + iced::widget::text::Catalog,
        <Theme as iced::widget::container::Catalog>::Class<'a>:
            From<iced::widget::container::StyleFn<'a, Theme>>,
        <Theme as iced::widget::text::Catalog>::Class<'a>:
            From<iced::widget::text::StyleFn<'a, Theme>>,
        Renderer: iced::advanced::Renderer + iced::advanced::text::Renderer + 'a,
    {
        let config = TrafficLightsConfig::new(self.is_dark, self.is_focused);
        view_traffic_lights(config, state, on_action, on_group_hover)
    }

    /// Resolves resize direction and cursor at coordinate (px, py).
    #[must_use]
    pub fn hit_test_resize(
        &self,
        px: f32,
        py: f32,
    ) -> Option<(ResizeDirection, window::Direction, iced::mouse::Interaction)> {
        resolve_resize_at(&self.metrics, px, py)
    }

    /// One-shot setup and hardening of the host operating system window.
    pub fn setup_native_window(
        &self,
        handle: raw_window_handle::RawWindowHandle,
        corner_radius: f64,
    ) -> Option<crate::native::DesktopBlurTarget> {
        let appearance = if self.is_dark {
            crate::native::WindowAppearance::Dark
        } else {
            crate::native::WindowAppearance::Light
        };
        crate::native_setup::setup_native_window(
            handle,
            crate::native_setup::NativeWindowOptions::new()
                .with_appearance(appearance)
                .with_corner_radius(corner_radius),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_initialization_and_events() {
        let config = WindowChromeConfig::separate(32.0);
        let mut controller = WindowShellController::new(config, true);

        assert_eq!(controller.window_size, (900.0, 600.0));
        assert!(controller.is_dark);
        assert!(controller.is_focused);
        assert_eq!(controller.metrics.rim_insets.top, 2.0);

        // Handle Resized event
        let shell_event = controller.handle_resized(1000.0, 700.0);
        assert_eq!(
            shell_event,
            ShellEvent::Resized {
                width: 1000.0,
                height: 700.0
            }
        );
        assert_eq!(controller.window_size, (1000.0, 700.0));
        assert_eq!(controller.metrics.window_size, (1000.0, 700.0));

        // Fullscreen collapse test
        controller.set_window_state(WindowState::Fullscreen);
        assert_eq!(controller.metrics.rim_insets, crate::platform::Insets::ZERO);
    }
}
