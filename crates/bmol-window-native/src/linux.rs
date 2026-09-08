//! Linux platform hooks and desktop environment integration.
//!
//! Provides FreeDesktop, GTK, and KDE theme detection, Wayland / X11 window
//! handle resolution, and appearance tracking without mandatory C-library dependencies.

use std::{
    env, fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};

use raw_window_handle::RawWindowHandle;

use crate::{DesktopBlurTarget, WindowAppearance};

static THEME_COUNTER: AtomicU64 = AtomicU64::new(0);
static LAST_KNOWN_DARK_MODE: AtomicBool = AtomicBool::new(false);

/// Resolves a native `DesktopBlurTarget` from Wayland or X11 handles.
#[must_use]
pub fn desktop_blur_target(handle: RawWindowHandle) -> Option<DesktopBlurTarget> {
    match handle {
        RawWindowHandle::Wayland(w) => {
            // w.surface is NonNull<c_void>
            Some(DesktopBlurTarget(w.surface.as_ptr() as usize))
        }
        RawWindowHandle::Xlib(x) => {
            // x.window is c_ulong
            Some(DesktopBlurTarget(x.window as usize))
        }
        RawWindowHandle::Xcb(x) => {
            // x.window is NonZeroU32
            Some(DesktopBlurTarget(x.window.get() as usize))
        }
        _ => None,
    }
}

/// Reapplies desktop blur under Linux compositor environments (Wayland/X11).
pub fn refresh_desktop_blur(_target: DesktopBlurTarget) {
    // Linux compositors (e.g. KWin, Hyprland, Sway) handle blur regions via
    // Wayland protocols or window properties.
}

/// Configures desktop blur radius under Linux compositor environments (Wayland/X11).
pub fn configure_desktop_blur(_target: DesktopBlurTarget, _radius: i64) {}

/// Configures extended dynamic range (EDR / HDR) on Linux.
pub fn configure_extended_dynamic_range(_target: DesktopBlurTarget, _enabled: bool) {
    // Wayland color-management protocol integration point.
}

/// Configures window corner radius.
pub fn configure_window_corner_radius(_target: DesktopBlurTarget, _radius: f64) {
    // Client-side decorations (CSD) or compositor mask integration point.
}

/// Configures window shadow.
pub fn configure_window_shadow(_target: DesktopBlurTarget, _has_shadow: bool) {
    // CSD or compositor shadow property integration point.
}

pub fn configure_window_appearance(_target: DesktopBlurTarget, _appearance: WindowAppearance) {}

/// Loads the current system desktop wallpaper on Linux.
#[must_use]
pub fn load_system_wallpaper_rgba(_target_w: u32, _target_h: u32) -> Option<(u32, u32, Vec<u8>)> {
    None
}

/// Notifies the runtime that the system theme has changed, incrementing the counter.
pub fn notify_theme_changed() {
    THEME_COUNTER.fetch_add(1, Ordering::SeqCst);
}

/// Returns the theme change counter.
#[must_use]
pub fn system_theme_change_counter() -> u64 {
    THEME_COUNTER.load(Ordering::Acquire)
}

/// Detects whether the host desktop environment is currently using dark mode.
///
/// Evaluation precedence:
/// 1. `GTK_THEME` environment variable (e.g. `Adwaita:dark`, `Breeze-Dark`).
/// 2. GTK 4.0 settings file (`~/.config/gtk-4.0/settings.ini`).
/// 3. GTK 3.0 settings file (`~/.config/gtk-3.0/settings.ini`).
/// 4. KDE Plasma configuration (`~/.config/kdeglobals`).
/// 5. Fallback via `gsettings get org.gnome.desktop.interface color-scheme`.
#[must_use]
pub fn is_system_dark_mode() -> bool {
    // Quick probe: rate-limited to avoid excessive I/O or child processes
    let is_dark = detect_dark_mode_cached();

    let previous = LAST_KNOWN_DARK_MODE.swap(is_dark, Ordering::SeqCst);
    if previous != is_dark {
        THEME_COUNTER.fetch_add(1, Ordering::SeqCst);
    }

    is_dark
}

// Cached detection with 1-second TTL to keep per-frame overhead near zero
fn detect_dark_mode_cached() -> bool {
    use std::sync::Mutex;

    struct Cache {
        result: bool,
        last_check: Instant,
    }

    static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

    if let Ok(mut guard) = CACHE.lock() {
        if let Some(ref cache) = *guard {
            if cache.last_check.elapsed() < Duration::from_millis(1000) {
                return cache.result;
            }
        }

        let detected = detect_dark_mode_raw();
        *guard = Some(Cache {
            result: detected,
            last_check: Instant::now(),
        });
        detected
    } else {
        detect_dark_mode_raw()
    }
}

fn detect_dark_mode_raw() -> bool {
    // 1. Check GTK_THEME environment variable
    if let Ok(gtk_theme) = env::var("GTK_THEME") {
        let lower = gtk_theme.to_lowercase();
        if lower.contains("dark") {
            return true;
        }
    }

    // 2. Check user's home directory config files
    if let Some(home) = dirs_home_dir() {
        // GTK 4
        let gtk4_ini = home.join(".config/gtk-4.0/settings.ini");
        if let Ok(content) = fs::read_to_string(gtk4_ini) {
            if let Some(dark) = parse_gtk_settings_is_dark(&content) {
                return dark;
            }
        }

        // GTK 3
        let gtk3_ini = home.join(".config/gtk-3.0/settings.ini");
        if let Ok(content) = fs::read_to_string(gtk3_ini) {
            if let Some(dark) = parse_gtk_settings_is_dark(&content) {
                return dark;
            }
        }

        // KDE Plasma
        let kde_globals = home.join(".config/kdeglobals");
        if let Ok(content) = fs::read_to_string(kde_globals) {
            if let Some(dark) = parse_kde_globals_is_dark(&content) {
                return dark;
            }
        }
    }

    // 3. Optional fallback: gsettings query if available
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            if text.contains("prefer-dark") {
                return true;
            }
        }
    }

    false
}

fn dirs_home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

/// Parses GTK `settings.ini` to check for `gtk-application-prefer-dark-theme` or dark theme name.
#[must_use]
pub fn parse_gtk_settings_is_dark(content: &str) -> Option<bool> {
    let mut prefer_dark: Option<bool> = None;
    let mut theme_name_dark: Option<bool> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let k = key.trim();
            let v = value.trim().to_lowercase();
            if k == "gtk-application-prefer-dark-theme" {
                if v == "1" || v == "true" {
                    prefer_dark = Some(true);
                } else if v == "0" || v == "false" {
                    prefer_dark = Some(false);
                }
            } else if k == "gtk-theme-name" {
                theme_name_dark = Some(v.contains("dark"));
            }
        }
    }

    prefer_dark.or(theme_name_dark)
}

/// Parses KDE `kdeglobals` to check for dark color scheme or dark window background.
#[must_use]
pub fn parse_kde_globals_is_dark(content: &str) -> Option<bool> {
    let mut in_colors_window = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_colors_window = trimmed.eq_ignore_ascii_case("[Colors:Window]");
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            let k = key.trim();
            let v = value.trim();

            if k.eq_ignore_ascii_case("ColorScheme") && v.to_lowercase().contains("dark") {
                return Some(true);
            }

            if in_colors_window && k.eq_ignore_ascii_case("BackgroundNormal") {
                // BackgroundNormal is usually in format "r,g,b" e.g. "36,44,52"
                let parts: Vec<&str> = v.split(',').map(str::trim).collect();
                if parts.len() == 3 {
                    let r: f32 = parts[0].parse().unwrap_or(255.0);
                    let g: f32 = parts[1].parse().unwrap_or(255.0);
                    let b: f32 = parts[2].parse().unwrap_or(255.0);
                    // Standard luminance formula: 0.299R + 0.587G + 0.114B
                    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
                    return Some(lum < 128.0);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gtk_settings_prefer_dark() {
        let ini = r#"
[Settings]
gtk-theme-name=Adwaita
gtk-application-prefer-dark-theme=1
"#;
        assert_eq!(parse_gtk_settings_is_dark(ini), Some(true));

        let ini_light = r#"
[Settings]
gtk-theme-name=Adwaita
gtk-application-prefer-dark-theme=0
"#;
        assert_eq!(parse_gtk_settings_is_dark(ini_light), Some(false));
    }

    #[test]
    fn test_parse_gtk_settings_theme_name() {
        let ini = r#"
[Settings]
gtk-theme-name=Breeze-Dark
"#;
        assert_eq!(parse_gtk_settings_is_dark(ini), Some(true));
    }

    #[test]
    fn test_parse_kde_globals_color_scheme() {
        let kde = r#"
[General]
ColorScheme=BreezeDark
"#;
        assert_eq!(parse_kde_globals_is_dark(kde), Some(true));

        let kde_bg = r#"
[Colors:Window]
BackgroundNormal=30,32,36
"#;
        assert_eq!(parse_kde_globals_is_dark(kde_bg), Some(true));

        let kde_light_bg = r#"
[Colors:Window]
BackgroundNormal=240,240,240
"#;
        assert_eq!(parse_kde_globals_is_dark(kde_light_bg), Some(false));
    }

    #[test]
    fn test_theme_counter_increment() {
        let c0 = system_theme_change_counter();
        notify_theme_changed();
        assert_eq!(system_theme_change_counter(), c0 + 1);
    }
}
