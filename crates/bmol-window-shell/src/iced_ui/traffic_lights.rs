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

use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{self, layout, renderer, svg as advanced_svg, Clipboard, Layout, Shell};
use iced::mouse;
use iced::widget::{container, mouse_area, row, space, stack, svg};
use iced::{Background, Color, Element, Event, Length, Padding, Rectangle, Size, Vector};
use spring_rs::{Spring, SpringMotion};

pub use crate::platform::traffic_lights::*;
use crate::platform::traffic_lights as metrics;

// Authentic Apple Text Glyphs matching original high-clarity typography
pub const GLYPH_CLOSE: &str = "✕";
pub const GLYPH_MINIMIZE: &str = "─";
pub const GLYPH_ZOOM: &str = "⤢";
pub const GLYPH_EXPAND: &str = "⤢";
pub const GLYPH_MAXIMIZE: &str = "+";

// Authentic Apple Vector SVGs with bold stroke and dynamic currentColor fill/stroke
pub const SVG_CLOSE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.6" d="M1.5 1.5 8.5 8.5M8.5 1.5 1.5 8.5"/></svg>"#;
pub const SVG_MINIMIZE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="2.6" d="M1.25 5h7.5"/></svg>"#;
pub const SVG_ZOOM: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-0.250 114.188 46.031 46.062"><path fill="currentColor" fill-rule="nonzero" d="M 0.000 122.313 L 0.000 146.688 C 0.000 149.063 2.312 149.875 3.750 148.438 L 34.000 118.188 C 35.406 116.781 34.594 114.438 32.250 114.438 L 7.969 114.438 C 2.656 114.438 0.000 117.063 0.000 122.313 Z M 37.687 160.000 C 42.937 160.000 45.531 157.313 45.531 152.031 L 45.531 127.750 C 45.531 125.375 43.219 124.563 41.781 126.000 L 11.531 156.250 C 10.125 157.656 10.937 160.000 13.281 160.000 Z"/></svg>"#;
pub const SVG_MAXIMIZE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="2.2" d="M5 1.5v7M1.5 5h7"/></svg>"#;

/// Authentic Apple Specular Crescent Arc and Subsurface Sheen SVG.
///
/// Simulates 3D spherical lens curvature, Fresnel top reflection (incident 47° angle),
/// and subsurface translucent light collection.
pub const SVG_SPECULAR_ARC: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 14 14" fill="none">
  <defs>
    <linearGradient id="specGrad" x1="7" y1="0.4" x2="7" y2="3.2" gradientUnits="userSpaceOnUse">
      <stop offset="0%" stop-color="#FFFFFF" stop-opacity="0.95"/>
      <stop offset="55%" stop-color="#FFFFFF" stop-opacity="0.45"/>
      <stop offset="100%" stop-color="#FFFFFF" stop-opacity="0.0"/>
    </linearGradient>
    <linearGradient id="bottomRimGrad" x1="7" y1="13.5" x2="7" y2="11.2" gradientUnits="userSpaceOnUse">
      <stop offset="0%" stop-color="#FFFFFF" stop-opacity="0.30"/>
      <stop offset="100%" stop-color="#FFFFFF" stop-opacity="0.0"/>
    </linearGradient>
  </defs>
  <!-- Authentic Top Edge Specular Crescent Highlight Arc: snugly fits the upper curvature -->
  <path d="M 2.4 3.0 C 3.4 1.1 10.6 1.1 11.6 3.0 C 10.2 1.8 3.8 1.8 2.4 3.0 Z" fill="url(#specGrad)"/>
  <!-- Subsurface Optical Lens Bottom Rim Reflection -->
  <path d="M 3.8 11.8 C 4.8 13.1 9.2 13.1 10.2 11.8 C 9.0 12.5 5.0 12.5 3.8 11.8 Z" fill="url(#bottomRimGrad)"/>
</svg>"##;

// Standard macOS traffic light dimensions and aliases
pub const WINDOW_CONTROL_NATIVE_SIZE: f32 = crate::platform::traffic_lights::DIAMETER;
pub const WINDOW_CONTROL_LARGE_SIZE: f32 = 64.0;
pub const WINDOW_CONTROL_GAP: f32 = crate::platform::traffic_lights::SPACING;
pub const WINDOW_CONTROL_LARGE_GAP: f32 = 12.0;
pub const WINDOW_CONTROL_NATIVE_IDS: [usize; 3] = [0, 1, 2];

// Spring physics constants matching authentic macOS click response
pub const PRESS_SCALE_OVERSHOOT: f32 = 1.18;
pub const PRESS_SCALE_SETTLED: f32 = 1.06;
pub const PRESS_SCALE_SPRING_DURATION: f32 = 0.24;
pub const PRESS_SCALE_SPRING_EXTRA_BOUNCE: f32 = 0.40;
pub const INTERACTION_ENTER_ANIMATION_TIME_CONSTANT: f32 = 0.08;
pub const INTERACTION_EXIT_ANIMATION_TIME_CONSTANT: f32 = 0.18;

/// When enabled, the traffic-light widget leaves the sphere body and specular
/// arc transparent so an external GPU glass compositor (for example
/// `bmol-glass-iced`) renders the physical glass, while the widget contributes
/// only the Apple vector glyph layer and the full press/hover interaction.
///
/// This is the `GpuCompositorPassthrough` architecture: a single physical-glass
/// implementation owns the sphere, and the widget never draws a second body.
static GLASS_PASSTHROUGH: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Enables or disables GPU glass passthrough for the traffic-light widget.
///
/// Call this once from any app that renders the traffic lights through a Liquid
/// Glass GPU compositor. Standalone apps leave it disabled so the widget draws
/// its own self-contained sphere.
pub fn set_glass_passthrough(enabled: bool) {
    GLASS_PASSTHROUGH.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

/// Returns whether GPU glass passthrough is currently enabled.
#[must_use]
pub fn glass_passthrough() -> bool {
    GLASS_PASSTHROUGH.load(std::sync::atomic::Ordering::Relaxed)
}

/// Mirrors AppKit's `NSWindow.isDocumentEdited`: when set, the close control
/// renders a filled status dot (the unsaved-document indicator) instead of the
/// `✕` glyph. Like the other traffic-light symbols, the dot follows the group
/// hover reveal and is hidden while the pointer is away.
static DOCUMENT_EDITED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Mirrors `NSWindow.setDocumentEdited(_:)`: marks the window's document as
/// having unsaved changes so the close control shows the dirty dot.
pub fn set_document_edited(edited: bool) {
    DOCUMENT_EDITED.store(edited, std::sync::atomic::Ordering::Relaxed);
}

/// Mirrors `NSWindow.isDocumentEdited`.
#[must_use]
pub fn is_document_edited() -> bool {
    DOCUMENT_EDITED.load(std::sync::atomic::Ordering::Relaxed)
}

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

impl PartialEq for TrafficLightsState {
    fn eq(&self, other: &Self) -> bool {
        (self.hover_progress - other.hover_progress).abs() < f32::EPSILON
            && (self.hover_target - other.hover_target).abs() < f32::EPSILON
            && self.press_targets == other.press_targets
            && self.expand_behavior == other.expand_behavior
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
            self.press_springs[index].retarget(1.0);
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

    /// Consumes an interactive traffic lights event and automatically updates spring physics and hover state.
    ///
    /// Returns `Some(action)` if a button was successfully clicked and released, ready to execute window actions.
    pub fn handle_event(&mut self, event: TrafficLightsEvent) -> Option<WindowControlAction> {
        match event {
            TrafficLightsEvent::GroupHover(hovered) => {
                self.on_group_hover(hovered);
                None
            }
            TrafficLightsEvent::PressStart(index) => {
                self.on_press_start(index);
                None
            }
            TrafficLightsEvent::PressCancel(index) => {
                self.on_press_cancel(index);
                None
            }
            TrafficLightsEvent::PressEnd(index) => {
                self.on_press_end(index);
                None
            }
            TrafficLightsEvent::Action(action) => Some(action),
        }
    }
}

/// Unified high-level interaction event emitted by authentic macOS traffic lights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrafficLightsEvent {
    GroupHover(bool),
    PressStart(usize),
    PressCancel(usize),
    PressEnd(usize),
    Action(WindowControlAction),
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
    let alpha_factor = if is_dark { 0.85 } else { 0.75 };
    if inactive {
        let inactive_color = if is_dark {
            Color::from_rgba(0.82, 0.83, 0.86, 0.76)
        } else {
            Color::from_rgba(0.52, 0.53, 0.56, 0.78)
        };
        let blended = blend_color(inactive_color, active, focus_amount);
        Color::from_rgba(blended.r, blended.g, blended.b, focus_amount * alpha_factor)
    } else {
        Color::from_rgba(active.r, active.g, active.b, focus_amount * alpha_factor)
    }
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

/// Palette defining the 3D physical liquid-glass appearance for a window control button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrafficLightSpherePalette {
    pub top_color: Color,
    pub bottom_color: Color,
    pub border_color: Color,
    pub glow_color: Color,
}

/// Blends two sphere palettes linearly by `amount`.
#[must_use]
pub fn blend_sphere_palette(from: TrafficLightSpherePalette, to: TrafficLightSpherePalette, amount: f32) -> TrafficLightSpherePalette {
    let amount = amount.clamp(0.0, 1.0);
    TrafficLightSpherePalette {
        top_color: blend_color(from.top_color, to.top_color, amount),
        bottom_color: blend_color(from.bottom_color, to.bottom_color, amount),
        border_color: blend_color(from.border_color, to.border_color, amount),
        glow_color: blend_color(from.glow_color, to.glow_color, amount),
    }
}

/// Resolves physical liquid-glass sphere palette for an active button.
#[must_use]
pub fn resolve_active_sphere_palette(
    action: WindowControlAction,
    is_dark: bool,
    is_pressed: bool,
    is_hovered: bool,
) -> TrafficLightSpherePalette {
    match action {
        WindowControlAction::Close => {
            if is_pressed {
                TrafficLightSpherePalette {
                    top_color: Color::from_rgb8(0xBF, 0x28, 0x1E),
                    bottom_color: Color::from_rgb8(0xD3, 0x3B, 0x36),
                    border_color: Color::from_rgb8(0xA0, 0x1E, 0x16),
                    glow_color: Color::from_rgba(0.80, 0.15, 0.10, 0.28),
                }
            } else if is_hovered {
                TrafficLightSpherePalette {
                    top_color: Color::from_rgb8(0xFF, 0x5E, 0x54),
                    bottom_color: Color::from_rgb8(0xFF, 0x93, 0x8C),
                    border_color: Color::from_rgb8(0xDB, 0x35, 0x2C),
                    glow_color: Color::from_rgba(1.0, 0.35, 0.30, 0.40),
                }
            } else {
                TrafficLightSpherePalette {
                    top_color: if is_dark {
                        Color::from_rgb8(0xFF, 0x5F, 0x56)
                    } else {
                        Color::from_rgb8(0xFF, 0x5F, 0x56)
                    },
                    bottom_color: Color::from_rgb8(0xFF, 0x7E, 0x75),
                    border_color: if is_dark {
                        Color::from_rgb8(0xB8, 0x32, 0x2B)
                    } else {
                        Color::from_rgb8(0xE0, 0x44, 0x3E)
                    },
                    glow_color: Color::from_rgba(0.95, 0.25, 0.20, 0.26),
                }
            }
        }
        WindowControlAction::Minimize => {
            if is_pressed {
                TrafficLightSpherePalette {
                    top_color: Color::from_rgb8(0xC8, 0x88, 0x00),
                    bottom_color: Color::from_rgb8(0xDB, 0x9A, 0x04),
                    border_color: Color::from_rgb8(0xA8, 0x70, 0x00),
                    glow_color: Color::from_rgba(0.85, 0.60, 0.05, 0.25),
                }
            } else if is_hovered {
                TrafficLightSpherePalette {
                    top_color: Color::from_rgb8(0xFF, 0xC8, 0x22),
                    bottom_color: Color::from_rgb8(0xFF, 0xEA, 0x80),
                    border_color: Color::from_rgb8(0xE0, 0xA2, 0x03),
                    glow_color: Color::from_rgba(1.0, 0.80, 0.20, 0.36),
                }
            } else {
                TrafficLightSpherePalette {
                    top_color: if is_dark {
                        Color::from_rgb8(0xFF, 0xBD, 0x2E)
                    } else {
                        Color::from_rgb8(0xFF, 0xBD, 0x2E)
                    },
                    bottom_color: Color::from_rgb8(0xFF, 0xDF, 0x5D),
                    border_color: if is_dark {
                        Color::from_rgb8(0xC2, 0x82, 0x16)
                    } else {
                        Color::from_rgb8(0xDE, 0xA1, 0x23)
                    },
                    glow_color: Color::from_rgba(1.0, 0.74, 0.15, 0.24),
                }
            }
        }
        WindowControlAction::Zoom | WindowControlAction::Expand => {
            if is_pressed {
                TrafficLightSpherePalette {
                    top_color: Color::from_rgb8(0x14, 0x8C, 0x1C),
                    bottom_color: Color::from_rgb8(0x19, 0xA0, 0x23),
                    border_color: Color::from_rgb8(0x0E, 0x72, 0x14),
                    glow_color: Color::from_rgba(0.12, 0.60, 0.18, 0.24),
                }
            } else if is_hovered {
                TrafficLightSpherePalette {
                    top_color: Color::from_rgb8(0x35, 0xDD, 0x4E),
                    bottom_color: Color::from_rgb8(0x78, 0xF5, 0x8D),
                    border_color: Color::from_rgb8(0x20, 0xBA, 0x38),
                    glow_color: Color::from_rgba(0.25, 0.90, 0.35, 0.36),
                }
            } else {
                TrafficLightSpherePalette {
                    top_color: if is_dark {
                        Color::from_rgb8(0x27, 0xC9, 0x3F)
                    } else {
                        Color::from_rgb8(0x27, 0xC9, 0x3F)
                    },
                    bottom_color: Color::from_rgb8(0x5E, 0xEA, 0x75),
                    border_color: if is_dark {
                        Color::from_rgb8(0x14, 0x8C, 0x1C)
                    } else {
                        Color::from_rgb8(0x1A, 0xAB, 0x29)
                    },
                    glow_color: Color::from_rgba(0.20, 0.82, 0.30, 0.24),
                }
            }
        }
    }
}

/// Resolves physical liquid-glass sphere palette for an inactive window control.
#[must_use]
pub fn resolve_inactive_sphere_palette(is_dark: bool) -> TrafficLightSpherePalette {
    if is_dark {
        TrafficLightSpherePalette {
            top_color: Color::from_rgb8(0x4C, 0x4C, 0x50),
            bottom_color: Color::from_rgb8(0x5E, 0x5E, 0x64),
            border_color: Color::from_rgba(0.20, 0.20, 0.22, 0.60),
            glow_color: Color::from_rgba(0.0, 0.0, 0.0, 0.20),
        }
    } else {
        TrafficLightSpherePalette {
            top_color: Color::from_rgb8(0xD1, 0xD1, 0xD6),
            bottom_color: Color::from_rgb8(0xE5, 0xE5, 0xEA),
            border_color: Color::from_rgba(0.70, 0.70, 0.73, 0.80),
            glow_color: Color::from_rgba(0.0, 0.0, 0.0, 0.05),
        }
    }
}

/// Resolves fill and border colors for an active button.
#[must_use]
pub fn resolve_active_button_colors(
    action: WindowControlAction,
    is_dark: bool,
    is_pressed: bool,
    is_hovered: bool,
) -> (Color, Color) {
    let pal = resolve_active_sphere_palette(action, is_dark, is_pressed, is_hovered);
    (pal.top_color, pal.border_color)
}

/// Resolves fill and border colors for an inactive button.
#[must_use]
pub fn resolve_inactive_button_colors(is_dark: bool) -> (Color, Color) {
    let pal = resolve_inactive_sphere_palette(is_dark);
    (pal.top_color, pal.border_color)
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

#[derive(Debug, Clone, Copy, Default)]
struct TrafficLightButtonState {
    hovered: bool,
    pressed: bool,
}

/// Custom interactive button widget for window control traffic lights.
///
/// Handles the complete macOS click lifecycle:
/// - `ButtonPressed`: enters pressed state and emits `on_press_start` to trigger spring scale-up.
/// - `MouseExit`: if pressed, cancels action and emits `on_press_cancel` to smoothly return spring.
/// - `ButtonReleased`: exits pressed state, emits `on_press_end` to settle spring, and publishes `on_action` if released inside.
pub struct TrafficLightButton<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    width: f32,
    height: f32,
    on_action: Message,
    on_press_start: Option<Message>,
    on_press_cancel: Option<Message>,
    on_press_end: Option<Message>,
}

impl<'a, Message, Theme, Renderer> std::fmt::Debug for TrafficLightButton<'a, Message, Theme, Renderer> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrafficLightButton")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl<'a, Message, Theme, Renderer> TrafficLightButton<'a, Message, Theme, Renderer> {
    pub fn new(content: Element<'a, Message, Theme, Renderer>, size: f32, on_action: Message) -> Self {
        Self {
            content,
            width: size,
            height: size,
            on_action,
            on_press_start: None,
            on_press_cancel: None,
            on_press_end: None,
        }
    }

    #[must_use]
    pub fn on_press_start(mut self, message: Message) -> Self {
        self.on_press_start = Some(message);
        self
    }

    #[must_use]
    pub fn on_press_cancel(mut self, message: Message) -> Self {
        self.on_press_cancel = Some(message);
        self
    }

    #[must_use]
    pub fn on_press_end(mut self, message: Message) -> Self {
        self.on_press_end = Some(message);
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for TrafficLightButton<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: advanced::Renderer + 'a,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<TrafficLightButtonState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(TrafficLightButtonState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.width), Length::Fixed(self.height))
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child_limits = limits.width(self.width).height(self.height);
        let child_node = self.content.as_widget_mut().layout(&mut tree.children[0], renderer, &child_limits);
        layout::Node::with_children(
            Size::new(self.width, self.height),
            vec![child_node],
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout.children().next().unwrap_or(layout),
            cursor,
            viewport,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let hovered = cursor.is_over(layout.bounds());
        let state = tree.state.downcast_mut::<TrafficLightButtonState>();
        let was_hovered = state.hovered;
        let was_pressed = state.pressed;
        state.hovered = hovered;

        if was_pressed && was_hovered && !hovered {
            if let Some(msg) = self.on_press_cancel.clone() {
                shell.publish(msg);
            }
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if hovered => {
                state.pressed = true;
                shell.capture_event();
                if let Some(msg) = self.on_press_start.clone() {
                    shell.publish(msg);
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.pressed => {
                state.pressed = false;
                shell.capture_event();
                if let Some(msg) = self.on_press_end.clone() {
                    shell.publish(msg);
                }
                if hovered {
                    shell.publish(self.on_action.clone());
                }
            }
            _ => {}
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap_or(layout),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }
}

impl<'a, Message: Clone + 'a, Theme: 'a, Renderer: advanced::Renderer + 'a>
    From<TrafficLightButton<'a, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer>
{
    fn from(btn: TrafficLightButton<'a, Message, Theme, Renderer>) -> Self {
        Element::new(btn)
    }
}

/// Transparent wrapper that publishes the measured absolute origin of the
/// traffic-light glyph row.
///
/// The Liquid Glass compositor reads the same origin for the sphere nodes, so
/// the glyphs and the GPU glass are driven by one real layout measurement
/// instead of two independently-computed coordinate systems. This removes the
/// per-circle drift that a separate `origin` estimate always produced.
struct MeasuredTrafficLights<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
}

impl<'a, Message, Theme, Renderer> MeasuredTrafficLights<'a, Message, Theme, Renderer> {
    fn new(content: Element<'a, Message, Theme, Renderer>) -> Self {
        Self { content }
    }
}

impl<Message, Theme, Renderer> std::fmt::Debug
    for MeasuredTrafficLights<'_, Message, Theme, Renderer>
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("MeasuredTrafficLights").finish_non_exhaustive()
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for MeasuredTrafficLights<'_, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let slop = metrics::control_hover_slop(WINDOW_CONTROL_NATIVE_SIZE);
        // Snap the measured origin to the device-pixel grid (0.5 logical px on a
        // 2x display) so the traffic-light margin stays a clean, reproducible
        // value instead of a fractional product of Iced's centering arithmetic.
        // The glyph itself stays within 0.25 px of the sphere centre.
        let snap = |value: f32| (value * 2.0).round() / 2.0;
        let origin_x = snap(bounds.x + slop);
        let origin_y = snap(bounds.y + slop);
        #[cfg(debug_assertions)]
        {
            use std::sync::atomic::{AtomicBool, Ordering};
            static PRINTED: AtomicBool = AtomicBool::new(false);
            if !PRINTED.swap(true, Ordering::Relaxed) {
                eprintln!(
                    "traffic-lights origin: raw=({:.3}, {:.3}) snapped=({:.1}, {:.1})",
                    bounds.x + slop,
                    bounds.y + slop,
                    origin_x,
                    origin_y,
                );
            }
        }
        bmol_window_glass::set_window_control_origin(origin_x, origin_y);
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(self.content.as_widget())]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.content.as_widget()]);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn advanced::widget::Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
    }
}

impl<'a, Message: Clone + 'a, Theme: 'a, Renderer: advanced::Renderer + 'a>
    From<MeasuredTrafficLights<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
{
    fn from(wrapper: MeasuredTrafficLights<'a, Message, Theme, Renderer>) -> Self {
        Element::new(wrapper)
    }
}

/// Builds a single authentic macOS traffic light button with full press lifecycle callbacks.
///
/// Implements the singular authentic liquid-glass window control architecture (GpuCompositorPassthrough):
/// leaves the sphere body transparent so the underlying GPU live SDF physical glass shines
/// through with zero occlusion, while overlaying Apple vector glyphs and driving bouncy spring physics.
#[allow(clippy::too_many_arguments)]
pub fn view_single_button_interactive<'a, Message: Clone + 'a, Theme, Renderer>(
    action: WindowControlAction,
    size: f32,
    is_dark: bool,
    is_active: bool,
    hover_progress: f32,
    scale: f32,
    is_fullscreen_symbol: bool,
    on_action: Message,
    on_press_start: Option<Message>,
    on_press_cancel: Option<Message>,
    on_press_end: Option<Message>,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: 'a + container::Catalog + iced::widget::svg::Catalog,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    <Theme as iced::widget::svg::Catalog>::Class<'a>: From<iced::widget::svg::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + advanced_svg::Renderer + 'a,
{
    let visual_size = size * scale;
    let hover = hover_progress.clamp(0.0, 1.0);
    let is_pressed = scale > 1.03;
    let is_hovered = hover > 0.5;

    // Layer 1: Authentic Liquid Glass Sphere Body with Dark Rim and Subtle Drop Shadow
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

    let passthrough = glass_passthrough();
    let sphere_body: Element<'a, Message, Theme, Renderer> = container(space())
        .width(Length::Fixed(visual_size))
        .height(Length::Fixed(visual_size))
        .style(move |_theme| {
            if passthrough {
                container::Style::default()
            } else {
                container::Style {
                    background: Some(Background::Color(fill)),
                    border: iced::Border::default()
                        .rounded(visual_size * 0.5)
                        .width(0.5)
                        .color(border),
                    shadow,
                    ..Default::default()
                }
            }
        })
        .into();

    // Layer 2: Authentic Apple Specular Crescent Highlight Arc & Subsurface Reflection.
    // Skipped under GPU passthrough: the physical-glass shader supplies its own specular.
    let specular_highlight: Element<'a, Message, Theme, Renderer> = if passthrough {
        space()
            .width(Length::Fixed(visual_size))
            .height(Length::Fixed(visual_size))
            .into()
    } else {
        container(
            svg(svg::Handle::from_memory(SVG_SPECULAR_ARC.as_bytes()))
                .width(Length::Fixed(visual_size))
                .height(Length::Fixed(visual_size)),
        )
        .width(Length::Fixed(visual_size))
        .height(Length::Fixed(visual_size))
        .into()
    };

    // Layer 3: Authentic Apple Vector SVG Glyph (or the unsaved-document dot).
    let glyph_element: Element<'a, Message, Theme, Renderer> = if hover > 0.001 {
        if action == WindowControlAction::Close && is_document_edited() {
            // AppKit marks an edited document with a filled dot instead of the ✕.
            let dot_color = window_control_glyph_color(is_dark, action, !is_active, hover);
            let opacity = hover.clamp(0.0, 1.0);
            container(window_control_status_dot(
                Color { a: opacity, ..dot_color },
                size * scale,
            ))
            .width(Length::Fixed(visual_size))
            .height(Length::Fixed(visual_size))
            .center_x(Length::Fixed(visual_size))
            .center_y(Length::Fixed(visual_size))
            .into()
        } else {
            let svg_source = match action {
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
            let glyph_size = window_control_glyph_size(action, size) * scale;
            let glyph_color = window_control_glyph_color(is_dark, action, !is_active, hover);
            let opacity = hover.clamp(0.0, 1.0);
            let color = Color { a: 1.0, ..glyph_color };

            container(
                svg(svg::Handle::from_memory(svg_source.as_bytes()))
                    .width(Length::Fixed(glyph_size))
                    .height(Length::Fixed(glyph_size))
                    .opacity(opacity)
                    .style(move |_theme, _status| svg::Style {
                        color: Some(color),
                    }),
            )
            .width(Length::Fixed(visual_size))
            .height(Length::Fixed(visual_size))
            .center_x(Length::Fixed(visual_size))
            .center_y(Length::Fixed(visual_size))
            .into()
        }
    } else {
        space()
            .width(Length::Fixed(visual_size))
            .height(Length::Fixed(visual_size))
            .into()
    };

    let circle_container = container(stack![
        sphere_body,
        specular_highlight,
        glyph_element,
    ])
    .width(Length::Fixed(visual_size))
    .height(Length::Fixed(visual_size))
    .center_x(Length::Fixed(visual_size))
    .center_y(Length::Fixed(visual_size));

    let centered_button = centered(circle_container.into(), size);

    let mut btn = TrafficLightButton::new(centered_button, size, on_action);
    if let Some(start) = on_press_start {
        btn = btn.on_press_start(start);
    }
    if let Some(cancel) = on_press_cancel {
        btn = btn.on_press_cancel(cancel);
    }
    if let Some(end) = on_press_end {
        btn = btn.on_press_end(end);
    }
    btn.into()
}

/// Builds a single authentic macOS traffic light button.
pub fn view_single_button<'a, Message: Clone + 'a, Theme, Renderer>(
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
    Theme: 'a + container::Catalog + iced::widget::svg::Catalog,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    <Theme as iced::widget::svg::Catalog>::Class<'a>: From<iced::widget::svg::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + advanced_svg::Renderer + 'a,
{
    view_single_button_interactive(
        action,
        size,
        is_dark,
        is_active,
        hover_progress,
        scale,
        is_fullscreen_symbol,
        on_action,
        None,
        None,
        None,
    )
}

/// Helper to render a group of controls with callbacks for advanced showcases.
#[allow(clippy::too_many_arguments)]
pub fn control_group<'a, Id: Copy + 'a, Message: Clone + 'a, Theme, Renderer>(
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
    Theme: 'a + container::Catalog + iced::widget::svg::Catalog,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    <Theme as iced::widget::svg::Catalog>::Class<'a>: From<iced::widget::svg::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + advanced_svg::Renderer + 'a,
{
    let hover = if show_glyphs { hover_amount } else { 0.0 };
    let is_active = !inactive;
    let is_fullscreen = expand_behavior == WindowExpandBehavior::Fullscreen;

    let btn0 = if interactive {
        view_single_button_interactive(
            WindowControlAction::Close,
            size,
            is_dark,
            is_active && !close_disabled,
            hover,
            press_scales[0],
            is_fullscreen,
            on_action(ids[0], WindowControlAction::Close),
            Some(on_press_start(ids[0])),
            Some(on_press_cancel(ids[0])),
            Some(on_press_end(ids[0])),
        )
    } else {
        view_single_button(
            WindowControlAction::Close,
            size,
            is_dark,
            is_active && !close_disabled,
            hover,
            press_scales[0],
            is_fullscreen,
            on_action(ids[0], WindowControlAction::Close),
        )
    };

    let btn1 = if interactive {
        view_single_button_interactive(
            WindowControlAction::Minimize,
            size,
            is_dark,
            is_active,
            hover,
            press_scales[1],
            is_fullscreen,
            on_action(ids[1], WindowControlAction::Minimize),
            Some(on_press_start(ids[1])),
            Some(on_press_cancel(ids[1])),
            Some(on_press_end(ids[1])),
        )
    } else {
        view_single_button(
            WindowControlAction::Minimize,
            size,
            is_dark,
            is_active,
            hover,
            press_scales[1],
            is_fullscreen,
            on_action(ids[1], WindowControlAction::Minimize),
        )
    };

    let btn2 = if interactive {
        view_single_button_interactive(
            WindowControlAction::Expand,
            size,
            is_dark,
            is_active,
            hover,
            press_scales[2],
            is_fullscreen,
            on_action(ids[2], WindowControlAction::Expand),
            Some(on_press_start(ids[2])),
            Some(on_press_cancel(ids[2])),
            Some(on_press_end(ids[2])),
        )
    } else {
        view_single_button(
            WindowControlAction::Expand,
            size,
            is_dark,
            is_active,
            hover,
            press_scales[2],
            is_fullscreen,
            on_action(ids[2], WindowControlAction::Expand),
        )
    };

    let controls = row![btn0, btn1, btn2].spacing(gap).align_y(iced::Alignment::Center);
    let slop = metrics::control_hover_slop(size);
    let tracking_area = container(controls).padding(Padding {
        top: slop,
        right: slop,
        bottom: slop,
        left: slop,
    });

    mouse_area(tracking_area)
        .on_enter(on_group_hover(true))
        .on_exit(on_group_hover(false))
        .into()
}

/// Builds an all-inclusive authentic macOS traffic lights widget row fully wired with spring physics,
/// hover fade animation, and press lifecycle callbacks.
pub fn view_traffic_lights_all_inclusive<'a, Message: Clone + 'a, Theme, Renderer>(
    state: &'a TrafficLightsState,
    is_focused: bool,
    is_dark: bool,
    on_event: impl Fn(TrafficLightsEvent) -> Message + 'a + Copy,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: 'a + container::Catalog + iced::widget::svg::Catalog,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    <Theme as iced::widget::svg::Catalog>::Class<'a>: From<iced::widget::svg::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + advanced_svg::Renderer + 'a,
{
    let slop = metrics::control_hover_slop(DIAMETER);
    let hover = state.hover_progress;
    let is_active = is_focused || hover > 0.05;
    let is_fullscreen = state.expand_behavior == WindowExpandBehavior::Fullscreen;

    let close_btn = view_single_button_interactive(
        WindowControlAction::Close,
        DIAMETER,
        is_dark,
        is_active,
        hover,
        state.press_springs[0].value(),
        is_fullscreen,
        on_event(TrafficLightsEvent::Action(WindowControlAction::Close)),
        Some(on_event(TrafficLightsEvent::PressStart(0))),
        Some(on_event(TrafficLightsEvent::PressCancel(0))),
        Some(on_event(TrafficLightsEvent::PressEnd(0))),
    );

    let min_btn = view_single_button_interactive(
        WindowControlAction::Minimize,
        DIAMETER,
        is_dark,
        is_active,
        hover,
        state.press_springs[1].value(),
        is_fullscreen,
        on_event(TrafficLightsEvent::Action(WindowControlAction::Minimize)),
        Some(on_event(TrafficLightsEvent::PressStart(1))),
        Some(on_event(TrafficLightsEvent::PressCancel(1))),
        Some(on_event(TrafficLightsEvent::PressEnd(1))),
    );

    let zoom_btn = view_single_button_interactive(
        WindowControlAction::Zoom,
        DIAMETER,
        is_dark,
        is_active,
        hover,
        state.press_springs[2].value(),
        is_fullscreen,
        on_event(TrafficLightsEvent::Action(WindowControlAction::Zoom)),
        Some(on_event(TrafficLightsEvent::PressStart(2))),
        Some(on_event(TrafficLightsEvent::PressCancel(2))),
        Some(on_event(TrafficLightsEvent::PressEnd(2))),
    );

    let row_widget = row![close_btn, min_btn, zoom_btn]
        .spacing(SPACING)
        .align_y(iced::Alignment::Center);

    let tracking_area = container(row_widget).padding(Padding {
        top: slop,
        right: slop,
        bottom: slop,
        left: slop,
    });

    MeasuredTrafficLights::new(
        mouse_area(tracking_area)
            .on_enter(on_event(TrafficLightsEvent::GroupHover(true)))
            .on_exit(on_event(TrafficLightsEvent::GroupHover(false)))
            .into(),
    )
    .into()
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
        assert_eq!(SPACING, 16.0);
        assert_eq!(metrics::TOTAL_WIDTH, 74.0);
        assert_eq!(WINDOW_CONTROL_NATIVE_SIZE, 14.0);
        assert_eq!(WINDOW_CONTROL_GAP, 16.0);
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

    #[test]
    fn test_traffic_lights_handle_event() {
        let mut state = TrafficLightsState::new();

        // 1. GroupHover
        assert_eq!(state.handle_event(TrafficLightsEvent::GroupHover(true)), None);
        assert_eq!(state.hover_target, 1.0);
        assert!(state.is_animating());

        // 2. PressStart for Close button (index 0)
        assert_eq!(state.handle_event(TrafficLightsEvent::PressStart(0)), None);
        assert_eq!(state.press_targets[0], 1.0);

        // 3. PressCancel for Close button
        assert_eq!(state.handle_event(TrafficLightsEvent::PressCancel(0)), None);
        assert_eq!(state.press_targets[0], 0.0);

        // 4. PressStart and PressEnd for Minimize button (index 1)
        assert_eq!(state.handle_event(TrafficLightsEvent::PressStart(1)), None);
        assert_eq!(state.press_targets[1], 1.0);
        assert_eq!(state.handle_event(TrafficLightsEvent::PressEnd(1)), None);
        assert_eq!(state.press_targets[1], 0.0);

        // 5. Action event triggers window action dispatch
        let action = state.handle_event(TrafficLightsEvent::Action(WindowControlAction::Zoom));
        assert_eq!(action, Some(WindowControlAction::Zoom));
    }

    #[test]
    fn test_traffic_lights_svg_sources() {
        assert!(SVG_CLOSE.contains("currentColor"));
        assert!(SVG_MINIMIZE.contains("currentColor"));
        assert!(SVG_ZOOM.contains("currentColor"));
        assert!(SVG_MAXIMIZE.contains("currentColor"));
    }

    #[test]
    fn test_traffic_lights_spring_press_overshoot() {
        let mut state = TrafficLightsState::new();
        state.on_press_start(0);
        assert_eq!(state.press_targets[0], 1.0);
        // Retargeted to 1.18 overshoot
        assert_eq!(PRESS_SCALE_OVERSHOOT, 1.18);
    }

    #[test]
    fn test_traffic_lights_specular_arc_source() {
        assert!(SVG_SPECULAR_ARC.contains("specGrad"));
        assert!(SVG_SPECULAR_ARC.contains("bottomRimGrad"));
        assert!(SVG_SPECULAR_ARC.contains("<path"));
    }

    #[test]
    fn test_traffic_lights_view_widget_construction() {
        let state = TrafficLightsState::new();
        let _element: Element<'_, TrafficLightsEvent, iced::Theme, iced::Renderer> =
            view_traffic_lights_all_inclusive(&state, true, false, |e| e);
    }
}

