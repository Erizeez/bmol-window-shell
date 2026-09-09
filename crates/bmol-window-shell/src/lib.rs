//! Modern seamless frameless window shell for desktop applications.
//!
//! # Scope
//!
//! `bmol-window-shell` owns exactly two concerns:
//!
//! 1. **Window rendering** — the non-client rim, corner curvature, drop shadow,
//!    backdrop blur, and the macOS traffic-light controls (including the GPU
//!    physical-glass material in `bmol-window-glass`).
//! 2. **Window layout** — the logical titlebar band (a full-width top region of
//!    height `H`, present even when unified chrome does not render it), the
//!    traffic-light band, content/sidebar rectangles, drag regions, and hit
//!    testing.
//!
//! All design tokens (metrics, geometry, colors, typography) live in
//! `bmol-designs`; this crate never hard-codes a design value. The titlebar
//! geometry is derived from the separate-mode height `H`: curvature is always
//! `H / 2`, the red control centre is `(H/2, H/2)`, and the traffic-light
//! spacing is `H / 2`.
//!
//! # Out of scope
//!
//! Application UI (menus, popovers, scroll views), asset/icon loading, wallpaper
//! capture, EDR configuration, and system-theme detection are not part of the
//! window shell. The convenience re-exports for those are demo helpers and may
//! move to dedicated crates.
//!
//! Exposes platform-neutral window models and macOS SkyLight/EDR native integrations.

pub use bmol_window_native as native;
pub use bmol_window_platform as platform;

pub use native::{
    DesktopBlurTarget, WindowAppearance, configure_desktop_blur, configure_window_appearance,
    configure_window_corner_radius, configure_window_shadow, desktop_blur_target,
    install_stage_manager_guard, refresh_desktop_blur,
};

/// Icon / asset loading helpers. Enable with the `assets` feature.
#[cfg(feature = "assets")]
pub use native::{
    app_icon_png, glyph_pdf, graphic_icon_png, named_asset_png, system_symbol_pdf,
};

/// System wallpaper capture. Enable with the `wallpaper` feature.
#[cfg(feature = "wallpaper")]
pub use native::{capture_desktop_backdrop, load_system_wallpaper_rgba};

/// Extended Dynamic Range configuration. Enable with the `edr` feature.
#[cfg(feature = "edr")]
pub use native::configure_extended_dynamic_range;

pub use native::is_system_dark_mode;

/// Legacy polling counter for system-appearance changes. Enable with the
/// `theme` feature; prefer `system_theme_subscription` instead.
#[cfg(feature = "theme")]
pub use native::system_theme_change_counter;

pub use platform::{
    BackdropError, BackdropFrame, BackdropFrameError, BackdropRequest, BackdropSize,
    BackdropSource, ChromeDrawPlan, ChromeLayoutMode, DesktopBackdropProvider, DisplayScale,
    Insets, Point, Rect, ResizeDirection, SidebarBackgroundConfig, SidebarBackgroundExtension,
    WindowChromeConfig, WindowChromeMetrics, WindowConfig, WindowHitZone, WindowState,
    physical_pixels_to_logical, snap_insets_to_physical, snap_to_physical_pixel,
    typography, window_metrics, window_rim,
};

#[cfg(not(feature = "iced"))]
pub use platform::traffic_lights;

pub mod native_setup;
pub use native_setup::{NativeWindowOptions, setup_native_window};

#[cfg(feature = "iced")]
pub mod iced_ui;

#[cfg(feature = "iced")]
pub use iced_ui::{
    ControlAction, ShellEvent, TrafficLightButton, TrafficLightsEvent, TrafficLightsState,
    WindowControlAction, WindowRimConfig, WindowShellController, glass_passthrough,
    is_document_edited, loyal_drag_bar, resize_direction_to_interaction,
    resize_direction_to_window_direction, resolve_resize_at, set_document_edited,
    set_glass_passthrough, traffic_lights, view_single_button, view_single_button_interactive,
    view_traffic_lights_all_inclusive, wrap_border_resizer, wrap_window_rim,
};

/// iced-based system appearance flow. Enable with the `theme` feature.
#[cfg(all(feature = "iced", feature = "theme"))]
pub use iced_ui::{initial_system_dark_mode, system_theme_subscription};

