//! High-level native window initialization and hardening hooks.
//!
//! Provides one-shot native setup to eliminate system borders, configure
//! Stage Manager re-blur guards, enable EDR, and sync appearance.

use raw_window_handle::RawWindowHandle;

use crate::native::{
    DesktopBlurTarget, WindowAppearance, configure_extended_dynamic_range,
    configure_window_appearance, configure_window_corner_radius, configure_window_shadow,
    desktop_blur_target, install_stage_manager_guard,
};

/// High-level native window initialization options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeWindowOptions {
    /// Desired window appearance mode (System, Light, Dark).
    pub appearance: WindowAppearance,
    /// Continuous squircle corner radius in points (defaults to 10.0).
    pub corner_radius: f64,
    /// Whether to enable system window drop shadow (defaults to true).
    pub enable_shadow: bool,
    /// Whether to configure extended dynamic range (defaults to true on Apple Silicon / HDR).
    pub enable_edr: bool,
    /// Whether to install Stage Manager / Mission Control compositor re-blur guards (defaults to true).
    pub install_stage_manager_guard: bool,
}

impl Default for NativeWindowOptions {
    fn default() -> Self {
        Self {
            appearance: WindowAppearance::System,
            corner_radius: 10.0,
            enable_shadow: true,
            enable_edr: true,
            install_stage_manager_guard: true,
        }
    }
}

impl NativeWindowOptions {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            appearance: WindowAppearance::System,
            corner_radius: 10.0,
            enable_shadow: true,
            enable_edr: true,
            install_stage_manager_guard: true,
        }
    }

    #[must_use]
    pub const fn with_appearance(mut self, appearance: WindowAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    #[must_use]
    pub const fn with_corner_radius(mut self, radius: f64) -> Self {
        self.corner_radius = radius;
        self
    }

    #[must_use]
    pub const fn with_shadow(mut self, enable_shadow: bool) -> Self {
        self.enable_shadow = enable_shadow;
        self
    }

    #[must_use]
    pub const fn with_system_shadow(self, enable_shadow: bool) -> Self {
        self.with_shadow(enable_shadow)
    }

    #[must_use]
    pub const fn with_edr(mut self, enable_edr: bool) -> Self {
        self.enable_edr = enable_edr;
        self
    }

    #[must_use]
    pub const fn with_stage_manager_guard(mut self, install: bool) -> Self {
        self.install_stage_manager_guard = install;
        self
    }
}

/// Hardens and configures a native window surface in a single call.
///
/// On macOS, this:
/// 1. Extracts the [`DesktopBlurTarget`] from the raw window handle.
/// 2. Sets native window appearance (`NSAppearanceNameAqua` / `NSAppearanceNameDarkAqua`).
/// 3. Sets window shadow behavior (clearing default outlines).
/// 4. Configures smooth squircle corner masking.
/// 5. Configures extended-linear sRGB / EDR floating point Metal drawable.
/// 6. Installs notification observers protecting against Stage Manager blur wipes.
///
/// On Linux, this configures compositor properties and window handles.
pub fn setup_native_window(
    handle: RawWindowHandle,
    options: NativeWindowOptions,
) -> Option<DesktopBlurTarget> {
    let target = desktop_blur_target(handle)?;

    configure_window_appearance(target, options.appearance);
    configure_window_shadow(target, options.enable_shadow);
    configure_window_corner_radius(target, options.corner_radius);
    configure_extended_dynamic_range(target, options.enable_edr);

    if options.install_stage_manager_guard {
        install_stage_manager_guard(target);
    }

    Some(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_window_options_builder() {
        let opts = NativeWindowOptions::new()
            .with_appearance(WindowAppearance::Dark)
            .with_corner_radius(12.0)
            .with_shadow(true)
            .with_edr(false)
            .with_stage_manager_guard(true);

        assert_eq!(opts.appearance, WindowAppearance::Dark);
        assert_eq!(opts.corner_radius, 12.0);
        assert!(opts.enable_shadow);
        assert!(!opts.enable_edr);
        assert!(opts.install_stage_manager_guard);
    }
}
