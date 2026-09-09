//! Application-level window shell state machine and runtime controller.
//!
//! Encapsulates metrics computation, display scale tracking, window state
//! transitions, and seamless wrapping for downstream applications.

use std::time::Instant;

use iced::{Element, Subscription, Task, window};

use crate::platform::{
    ChromeLayoutMode, ResizeDirection, WindowChromeConfig, WindowChromeMetrics, WindowState,
};

use super::resizer::resolve_resize_at;
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
    pub traffic_lights: super::traffic_lights::TrafficLightsState,
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
            traffic_lights: super::traffic_lights::TrafficLightsState::new(),
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
                self.traffic_lights.on_group_hover(false);
                Some(ShellEvent::Focused)
            }
            window::Event::Unfocused => {
                self.is_focused = false;
                self.traffic_lights.on_group_hover(false);
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

    /// Advances spring dynamics and hover transitions for the window controls.
    pub fn step(&mut self, now: Instant) {
        self.traffic_lights.step(now);
    }

    /// Returns true if any animation (hover transition or click spring) is currently active.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.traffic_lights.is_animating()
    }

    /// Yields an animation subscription for window frame updates if controls are animating.
    #[must_use]
    pub fn animation_subscription<Message: 'static>(
        &self,
        f: impl Fn(Instant) -> Message + 'static + Send + Sync + Clone,
    ) -> Subscription<Message> {
        if self.is_animating() {
            window::frames().map(f)
        } else {
            Subscription::none()
        }
    }

    /// Yields a subscription that fires whenever the operating system
    /// switches between light and dark mode.
    ///
    /// Wire this into `Subscription::batch` alongside `animation_subscription`
    /// and `window::events`. The controller processes the resulting message
    /// through [`handle_system_theme`] which updates `is_dark`, recomputes
    /// metrics, and returns a [`ShellEvent::ThemeChanged`] so the application
    /// can refresh its own theme and derived state.
    #[cfg(feature = "theme")]
    #[must_use]
    pub fn theme_subscription<Message: 'static>(
        &self,
        f: impl Fn(iced::theme::Mode) -> Message + 'static + Send + Sync + Clone,
    ) -> Subscription<Message> {
        super::system_theme_subscription(f)
    }

    /// Processes a system theme change, updating `is_dark`, recomputing
    /// chrome metrics, and returning the semantic event.
    ///
    /// Call this from the application's `update` when it receives the message
    /// produced by [`theme_subscription`].
    pub fn handle_system_theme(&mut self, mode: iced::theme::Mode) -> ShellEvent {
        let is_dark = mode == iced::theme::Mode::Dark;
        self.set_dark_mode(is_dark);
        ShellEvent::ThemeChanged { is_dark }
    }

    /// Dispatches a high-level `WindowControlAction` using the bound `window_id`.
    pub fn handle_control_action<Message: 'static>(
        &mut self,
        action: super::traffic_lights::WindowControlAction,
    ) -> Task<Message> {
        match action {
            super::traffic_lights::WindowControlAction::Close => {
                if let Some(id) = self.window_id {
                    window::close(id)
                } else {
                    Task::none()
                }
            }
            super::traffic_lights::WindowControlAction::Minimize => {
                if let Some(id) = self.window_id {
                    window::minimize(id, true)
                } else {
                    Task::none()
                }
            }
            super::traffic_lights::WindowControlAction::Zoom
            | super::traffic_lights::WindowControlAction::Expand => {
                if let Some(id) = self.window_id {
                    window::toggle_maximize(id)
                } else {
                    Task::none()
                }
            }
        }
    }

    /// Processes an interactive `TrafficLightsEvent`, updating internal physics and automatically
    /// executing window operations (close, minimize, maximize) if an action was triggered.
    pub fn handle_traffic_lights<Message: 'static>(
        &mut self,
        event: super::traffic_lights::TrafficLightsEvent,
    ) -> Task<Message> {
        if let Some(action) = self.traffic_lights.handle_event(event) {
            self.handle_control_action(action)
        } else {
            Task::none()
        }
    }

    /// Builds the pre-fabricated traffic lights widget bound to this window's theme and focus state.
    pub fn traffic_lights_view<'a, Message: Clone + 'a, Theme, Renderer>(
        &'a self,
        on_event: impl Fn(super::traffic_lights::TrafficLightsEvent) -> Message + 'a + Copy,
    ) -> Element<'a, Message, Theme, Renderer>
    where
        Theme: 'a + iced::widget::container::Catalog + iced::widget::svg::Catalog,
        <Theme as iced::widget::container::Catalog>::Class<'a>:
            From<iced::widget::container::StyleFn<'a, Theme>>,
        <Theme as iced::widget::svg::Catalog>::Class<'a>:
            From<iced::widget::svg::StyleFn<'a, Theme>>,
        Renderer: iced::advanced::Renderer + iced::advanced::svg::Renderer + 'a,
    {
        super::traffic_lights::view_traffic_lights_all_inclusive(
            &self.traffic_lights,
            self.is_focused,
            self.is_dark,
            on_event,
        )
    }

    /// Builds the traffic-light glyph + interaction overlay positioned at the
    /// exact GPU glass origin.
    ///
    /// The glass spheres and the glyphs then share one geometry source
    /// (`origin + index * (size + gap)`), so the symbols can never drift from
    /// the spheres the way a separate Iced layout would.
    ///
    /// Place the returned element in a full-size `Stack` above the window
    /// content (and route it through the compositor's overlay layer) instead
    /// of laying it out inside a header row.
    pub fn traffic_lights_overlay<'a, Message: Clone + 'a, Theme, Renderer>(
        &'a self,
        on_event: impl Fn(super::traffic_lights::TrafficLightsEvent) -> Message + 'a + Copy,
    ) -> Element<'a, Message, Theme, Renderer>
    where
        Theme: 'a + iced::widget::container::Catalog + iced::widget::svg::Catalog,
        <Theme as iced::widget::container::Catalog>::Class<'a>:
            From<iced::widget::container::StyleFn<'a, Theme>>,
        <Theme as iced::widget::svg::Catalog>::Class<'a>:
            From<iced::widget::svg::StyleFn<'a, Theme>>,
        Renderer: iced::advanced::Renderer + iced::advanced::svg::Renderer + 'a,
    {
        let (origin_x, origin_y) = bmol_window_glass::active_window_control_origin();
        super::traffic_lights::positioned_control_group(
            super::traffic_lights::view_traffic_lights_all_inclusive(
                &self.traffic_lights,
                self.is_focused,
                self.is_dark,
                on_event,
            ),
            origin_x,
            origin_y,
            super::traffic_lights::WINDOW_CONTROL_NATIVE_SIZE,
        )
    }

    /// Builds the traffic lights widget with custom external state if needed.
    pub fn traffic_lights_view_with_state<'a, Message: Clone + 'a, Theme, Renderer>(
        &self,
        state: &'a super::traffic_lights::TrafficLightsState,
        on_event: impl Fn(super::traffic_lights::TrafficLightsEvent) -> Message + 'a + Copy,
    ) -> Element<'a, Message, Theme, Renderer>
    where
        Theme: 'a + iced::widget::container::Catalog + iced::widget::svg::Catalog,
        <Theme as iced::widget::container::Catalog>::Class<'a>:
            From<iced::widget::container::StyleFn<'a, Theme>>,
        <Theme as iced::widget::svg::Catalog>::Class<'a>:
            From<iced::widget::svg::StyleFn<'a, Theme>>,
        Renderer: iced::advanced::Renderer + iced::advanced::svg::Renderer + 'a,
    {
        super::traffic_lights::view_traffic_lights_all_inclusive(
            state,
            self.is_focused,
            self.is_dark,
            on_event,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_initialization_and_events() {
        let config = WindowChromeConfig::separate(
            bmol_window_platform::window_metrics::COMPACT_TITLEBAR_HEIGHT,
        );
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

    #[test]
    fn test_controller_traffic_lights_lifecycle() {
        let config = WindowChromeConfig::separate(
            bmol_window_platform::window_metrics::COMPACT_TITLEBAR_HEIGHT,
        );
        let mut controller = WindowShellController::new(config, true);

        assert_eq!(controller.traffic_lights.hover_progress, 0.0);
        assert!(!controller.is_animating());

        // Event: group hover
        let _ = controller.handle_traffic_lights::<()>(super::super::TrafficLightsEvent::GroupHover(true));
        assert_eq!(controller.traffic_lights.hover_target, 1.0);
        assert!(controller.is_animating());

        // Focus loss automatically clears hover
        let shell_event = controller.handle_window_event(&window::Event::Unfocused);
        assert_eq!(shell_event, Some(ShellEvent::Unfocused));
        assert!(!controller.is_focused);
        assert_eq!(controller.traffic_lights.hover_target, 0.0);

        // Step animation
        controller.step(Instant::now());
    }
}
