//! Modern seamless frameless window shell for desktop applications.
//!
//! Exposes platform-neutral window models and macOS SkyLight/EDR native integrations.

pub use bmol_window_native as native;
pub use bmol_window_platform as platform;

pub use native::{
    DesktopBlurTarget, WindowAppearance, capture_desktop_backdrop, configure_extended_dynamic_range,
    configure_window_appearance, configure_window_corner_radius, configure_window_shadow,
    desktop_blur_target, graphic_icon_png, glyph_pdf, install_stage_manager_guard,
    is_system_dark_mode, named_asset_png, refresh_desktop_blur, system_symbol_pdf,
    system_theme_change_counter,
};

pub use platform::{
    BackdropError, BackdropFrame, BackdropFrameError, BackdropRequest, BackdropSize,
    BackdropSource, ChromeDrawPlan, ChromeLayoutMode, DesktopBackdropProvider, DisplayScale,
    Insets, Point, Rect, SidebarBackgroundConfig, SidebarBackgroundExtension, WindowChromeConfig,
    WindowChromeMetrics, WindowConfig, WindowHitZone, traffic_lights,
};
