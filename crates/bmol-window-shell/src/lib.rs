//! Modern seamless frameless window shell for desktop applications.
//!
//! Exposes platform-neutral window models and macOS SkyLight/EDR native integrations.

pub use bmol_window_native as native;
pub use bmol_window_platform as platform;

pub use native::{
    DesktopBlurTarget, capture_desktop_backdrop, configure_extended_dynamic_range,
    configure_window_corner_radius, desktop_blur_target, graphic_icon_png, glyph_pdf,
    install_stage_manager_guard, named_asset_png, refresh_desktop_blur, system_symbol_pdf,
};

pub use platform::{
    BackdropError, BackdropFrame, BackdropFrameError, BackdropRequest, BackdropSize,
    BackdropSource, ChromeDrawPlan, ChromeLayoutMode, DesktopBackdropProvider, DisplayScale,
    Insets, Point, Rect, SidebarBackgroundConfig, SidebarBackgroundExtension, WindowChromeConfig,
    WindowChromeMetrics, WindowConfig, WindowHitZone, traffic_lights,
};
