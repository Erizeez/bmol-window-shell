//! Window frame chrome layout, collision detection, and layout metrics.
//!
//! Provides two standard window chrome layouts:
//! 1. `ChromeLayoutMode::Separate`: Standalone titlebar separated from content below.
//! 2. `ChromeLayoutMode::Unified`: Integrated chrome where content / sidebar extends to the top edge.

use crate::geometry::{Insets, Point, Rect};

/// Default measurements for macOS-style traffic lights.
pub mod traffic_lights {
    /// Authentic macOS traffic light button diameter (strictly 14.0 pt / 28 px on 2x Retina).
    pub const DIAMETER: f32 = 14.0;
    /// Authentic macOS spacing between traffic light buttons (strictly 9.0 px).
    pub const SPACING: f32 = 9.0;
    /// Distance from the left window edge to the leftmost edge of the red light (strictly 10.0 px).
    pub const LEADING_MARGIN: f32 = 10.0;
    /// Total width across the three lights (14 + 9 + 14 + 9 + 14 = 60.0 px).
    pub const TOTAL_WIDTH: f32 = 60.0;
    /// Authentic height matches the button diameter (14.0 pt).
    pub const HEIGHT: f32 = DIAMETER;
    /// Clearance distance from the right edge of traffic lights to the first letter of title (strictly 15.0 px).
    pub const TITLE_CLEARANCE: f32 = 15.0;
    /// Recommended horizontal clearance width including padding (10 + 60 + 8 = 78.0 px).
    pub const EXCLUSION_WIDTH: f32 = 78.0;
}

/// Typography metrics and recommended Apple font families for window chrome.
pub mod typography {
    /// Apple standard window titlebar font size (13.0 pt).
    pub const TITLEBAR_FONT_SIZE: f32 = 13.0;

    /// Apple unified toolbar section header font size (14.0 pt).
    pub const UNIFIED_HEADER_FONT_SIZE: f32 = 14.0;

    /// Primary font family name for Apple's San Francisco on macOS (`"System Font"` / `".SF NS"`).
    pub const MACOS_SYSTEM_FONT: &str = "System Font";

    /// Alternative font family name for Apple's San Francisco on macOS (`".SF NS"`).
    pub const MACOS_SF_NS: &str = ".SF NS";

    /// Primary Chinese font family name for Apple's 苹方 (`"PingFang SC"`).
    pub const MACOS_PINGFANG_SC: &str = "PingFang SC";

    /// Standard standalone / Linux font family name for extracted SF Pro.
    pub const LINUX_SF_PRO_TEXT: &str = "SFNS Text";
}

/// Standard specifications for authentic macOS window rims and borders.
///
/// In Apple macOS (Big Sur through Sequoia):
/// - **Light Mode**: A crisp 1px subtle gray outer rim (`rgba(0, 0, 0, 0.10)`).
/// - **Dark Mode**: A dual-layer compound 2px rim:
///   1. Outer rim: 1px deep black line (`rgba(0, 0, 0, 0.85)`) to delineate from the shadow and desktop.
///   2. Inner rim: 1px subtle gray line (`rgb(70, 70, 70)`) providing crisp border depth.
pub mod window_rim {
    use crate::geometry::Insets;

    /// In Light mode: single 1px subtle gray outer rim.
    pub const LIGHT_RIM_WIDTH: f32 = 1.0;
    pub const LIGHT_RIM_COLOR_RGBA: (f32, f32, f32, f32) = (0.0, 0.0, 0.0, 0.10);

    /// In Dark mode outer layer: 1px deep black delineation rim.
    pub const DARK_OUTER_RIM_WIDTH: f32 = 1.0;
    pub const DARK_OUTER_RIM_COLOR_RGBA: (f32, f32, f32, f32) = (0.0, 0.0, 0.0, 0.85);

    /// In Dark mode inner layer: 1px subtle gray line (`rgb(70, 70, 70)`).
    pub const DARK_INNER_RIM_WIDTH: f32 = 1.0;
    pub const DARK_INNER_RIM_COLOR_RGB8: (u8, u8, u8) = (70, 70, 70);
    pub const DARK_INNER_RIM_COLOR_RGBA: (f32, f32, f32, f32) = (
        70.0 / 255.0,
        70.0 / 255.0,
        70.0 / 255.0,
        1.0,
    );

    /// Total physical compound rim thickness in dark mode (1px outer + 1px inner = 2.0px).
    pub const DARK_TOTAL_RIM_WIDTH: f32 = 2.0;

    /// Returns the non-client rim thickness for the specified theme mode.
    #[must_use]
    pub const fn rim_thickness(is_dark: bool) -> f32 {
        if is_dark {
            DARK_TOTAL_RIM_WIDTH
        } else {
            LIGHT_RIM_WIDTH
        }
    }

    /// Returns the non-client rim insets for the specified theme mode.
    #[must_use]
    pub const fn insets(is_dark: bool) -> Insets {
        let w = rim_thickness(is_dark);
        Insets::uniform(w)
    }
}


/// Standard window layout metrics, header heights, sidebar widths, and corner curvatures.
pub mod window_metrics {
    /// Modern macOS unified toolbar/header height (e.g. System Settings AXToolbar, strictly 52.0 pt).
    pub const FUSED_HEADER_HEIGHT: f32 = 52.0;

    /// Classic macOS standalone titlebar height (strictly 32.0 pt).
    pub const COMPACT_TITLEBAR_HEIGHT: f32 = 32.0;

    /// Comfortable standalone titlebar height with generous action clearance (38.0 pt).
    pub const COMFORTABLE_TITLEBAR_HEIGHT: f32 = 38.0;

    /// Modern macOS regular card-style sidebar width (232.0 pt, matching System Settings).
    pub const SIDEBAR_WIDTH_REGULAR: f32 = 232.0;

    /// Classic macOS compact sidebar width (220.0 pt).
    pub const SIDEBAR_WIDTH_COMPACT: f32 = 220.0;

    /// Recommended horizontal inset for sidebar items (10.0 pt).
    pub const SIDEBAR_CONTENT_INSET: f32 = 10.0;

    /// Standard continuous corner curvature radius for frameless windows (14.0 pt).
    pub const DEFAULT_CORNER_RADIUS: f32 = 14.0;
}

/// The layout mode for the window chrome / titlebar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChromeLayoutMode {
    /// Separate titlebar mode: The titlebar sits strictly above the content.
    /// Content starts at `y = titlebar_height`.
    Separate {
        /// Height of the standalone titlebar in logical points (typically 32.0 - 52.0).
        titlebar_height: f32,
    },
    /// Unified (integrated) chrome mode: Content or sidebar extends all the way
    /// to the top window edge, with titlebar and controls merged into the content.
    Unified {
        /// Reserved height for the top header / toolbar area (typically 44.0 - 64.0).
        header_height: f32,
        /// Width of the integrated sidebar, if present.
        sidebar_width: Option<f32>,
    },
}

impl ChromeLayoutMode {
    /// Standalone titlebar with standard height (32.0 pt).
    #[must_use]
    pub const fn separate_default() -> Self {
        Self::Separate {
            titlebar_height: window_metrics::COMPACT_TITLEBAR_HEIGHT,
        }
    }

    /// Unified chrome with standard header height (52.0 pt) and optional sidebar width (220.0 pt).
    #[must_use]
    pub const fn unified_default(sidebar_width: Option<f32>) -> Self {
        Self::Unified {
            header_height: window_metrics::FUSED_HEADER_HEIGHT,
            sidebar_width,
        }
    }

    /// The effective top chrome / header height.
    #[must_use]
    pub const fn header_height(self) -> f32 {
        match self {
            Self::Separate { titlebar_height } => titlebar_height,
            Self::Unified { header_height, .. } => header_height,
        }
    }
}

/// The window state (normal floating window, maximized, or native fullscreen).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WindowState {
    #[default]
    Normal,
    Maximized,
    Fullscreen,
}

/// Configuration parameters for window chrome calculation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowChromeConfig {
    pub mode: ChromeLayoutMode,
    /// The window state (normal, maximized, or native fullscreen).
    pub state: WindowState,
    /// Display scale factor (e.g. 2.0 for Retina, 1.25/1.5 for Linux fractional scaling).
    pub scale_factor: f32,
    /// Distance from the window left edge to the traffic lights.
    ///
    /// When `None` (the default), this is dynamically derived to match the vertical
    /// top margin (`tl_x == tl_y`), ensuring balanced square symmetry in the window corner.
    pub traffic_lights_leading: Option<f32>,
    /// Custom vertical offset for traffic lights, or None for automatic vertical centering.
    pub traffic_lights_top_offset: Option<f32>,
    /// Minimum width reserved for title text or toolbar actions.
    pub toolbar_action_reserved_width: f32,
}

impl Default for WindowChromeConfig {
    fn default() -> Self {
        Self {
            mode: ChromeLayoutMode::unified_default(Some(220.0)),
            state: WindowState::Normal,
            scale_factor: 1.0,
            traffic_lights_leading: None,
            traffic_lights_top_offset: None,
            toolbar_action_reserved_width: 160.0,
        }
    }
}

impl WindowChromeConfig {
    #[must_use]
    pub const fn separate(titlebar_height: f32) -> Self {
        Self {
            mode: ChromeLayoutMode::Separate { titlebar_height },
            state: WindowState::Normal,
            scale_factor: 1.0,
            traffic_lights_leading: None,
            traffic_lights_top_offset: None,
            toolbar_action_reserved_width: 140.0,
        }
    }

    #[must_use]
    pub const fn unified(header_height: f32, sidebar_width: Option<f32>) -> Self {
        Self {
            mode: ChromeLayoutMode::Unified {
                header_height,
                sidebar_width,
            },
            state: WindowState::Normal,
            scale_factor: 1.0,
            traffic_lights_leading: None,
            traffic_lights_top_offset: None,
            toolbar_action_reserved_width: 160.0,
        }
    }

    /// Creates a pure unified layout config specifying only the top header height.
    ///
    /// The page layout is completely owned and styled by the downstream application
    /// (single-pane, multi-column, or custom canvas). The shell provides collision
    /// bounds for traffic lights avoidance and ensures the entire top background
    /// faithfully handles window dragging.
    #[must_use]
    pub const fn unified_header(header_height: f32) -> Self {
        Self::unified(header_height, None)
    }

    /// Configures the window state (Normal, Maximized, Fullscreen).
    #[must_use]
    pub const fn with_state(mut self, state: WindowState) -> Self {
        self.state = state;
        self
    }

    /// Configures the display scale factor (e.g. 2.0 for Retina, 1.25/1.5 for fractional DPI).
    #[must_use]
    pub const fn with_scale_factor(mut self, scale_factor: f32) -> Self {
        self.scale_factor = scale_factor;
        self
    }

    /// Configures custom leading margin for traffic lights.
    #[must_use]
    pub const fn with_traffic_lights_leading(mut self, leading: f32) -> Self {
        self.traffic_lights_leading = Some(leading);
        self
    }

    /// Configures custom vertical offset for traffic lights.
    #[must_use]
    pub const fn with_traffic_lights_top_offset(mut self, offset: f32) -> Self {
        self.traffic_lights_top_offset = Some(offset);
        self
    }
}

/// Identifies which semantic zone a given coordinate hits within the window chrome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowHitZone {
    /// Within the non-client window rim / border area (1px light mode, 2px dark mode).
    /// This zone belongs strictly to system window framing and MUST NOT be occupied or intercepted
    /// by downstream application widgets.
    WindowRim,
    /// Within the traffic lights bounding box (Close / Miniaturize / Zoom).
    TrafficLights,
    /// Empty titlebar / header area where dragging the window is supported.
    DraggableChrome,
    /// Dedicated action or toolbar area (e.g. search box, buttons).
    ToolbarAction,
    /// Sidebar content area (in Unified mode, below or outside traffic lights exclusion).
    Sidebar,
    /// Main application content area.
    Content,
    /// Outside window bounds.
    Outside,
}

/// Comprehensive layout metrics and collision bounds computed for a specific window dimension.
///
/// Downstream applications (such as UI toolkits, widgets, or custom renderers) use this structure to:
/// 1. Confine application layout strictly within `safe_client_rect` to prevent rim override.
/// 2. Avoid overlapping interactive widgets with the traffic lights exclusion zone.
/// 3. Implement native-style window dragging without intercepting inner widget clicks.
/// 4. Size and position content views, sidebars, and titlebar separators correctly.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowChromeMetrics {
    /// Full window dimensions (width, height).
    pub window_size: (f32, f32),
    /// Active layout mode.
    pub mode: ChromeLayoutMode,
    /// Active window state (Normal, Maximized, Fullscreen).
    pub state: WindowState,
    /// Active display scale factor.
    pub scale_factor: f32,

    /// Non-client window rim insets (1.0 px light mode, 2.0 px dark mode; 0.0 in fullscreen).
    pub rim_insets: Insets,
    /// Safe client rectangle strictly inside the window rim.
    /// Downstream application views MUST be contained within this rectangle.
    pub safe_client_rect: Rect,

    /// Exact physical bounding box of the three traffic lights.
    pub traffic_lights_hitbox: Rect,
    /// Mandatory exclusion zone around the traffic lights.
    /// Downstream applications MUST NOT place interactive components in this area.
    pub traffic_lights_exclusion_zone: Rect,

    /// Full rectangle of the top header / titlebar region.
    pub header_rect: Rect,

    /// Regions where mouse clicks should initiate window dragging.
    pub drag_regions: Vec<Rect>,

    /// Safe rectangle for the main application content view.
    pub content_rect: Rect,

    /// Safe rectangle for the sidebar, if present.
    pub sidebar_rect: Option<Rect>,
    /// Sidebar content rectangle that has avoided the traffic lights exclusion zone.
    pub sidebar_safe_content_rect: Option<Rect>,

    /// Safe bounding box for drawing a centered window title.
    pub centered_title_rect: Option<Rect>,

    /// Safe bounding box for drawing a left-aligned window title (16px to the right of traffic lights).
    pub left_aligned_title_rect: Option<Rect>,

    /// Recommended safe insets for inner content.
    pub content_safe_insets: Insets,
}

impl WindowChromeMetrics {
    /// Computes the complete chrome metrics and collision map for given window dimensions.
    ///
    /// By default, assumes dark mode compound rim (2px) to guarantee safe insets under the
    /// strictest boundary condition. Use [`WindowChromeMetrics::compute_with_theme`] to
    /// explicitly configure light vs dark mode rim insets.
    #[must_use]
    pub fn compute(window_width: f32, window_height: f32, config: &WindowChromeConfig) -> Self {
        Self::compute_with_theme(window_width, window_height, config, true)
    }

    /// Computes chrome metrics with explicit theme awareness for non-client rim insets.
    #[must_use]
    pub fn compute_with_theme(
        window_width: f32,
        window_height: f32,
        config: &WindowChromeConfig,
        is_dark: bool,
    ) -> Self {
        let width = window_width.max(1.0);
        let height = window_height.max(1.0);

        // In fullscreen mode, outer non-client rims automatically collapse to zero
        let rim_insets = if config.state == WindowState::Fullscreen {
            Insets::ZERO
        } else {
            let base_insets = window_rim::insets(is_dark);
            if config.scale_factor > 0.0 {
                crate::geometry::snap_insets_to_physical(base_insets, config.scale_factor)
            } else {
                base_insets
            }
        };
        let safe_client_rect = Rect::new(0.0, 0.0, width, height).insets(rim_insets);

        let header_h = config.mode.header_height().min(height);

        // Traffic lights positioning:
        // By default, traffic lights are vertically centered within the header area.
        let tl_y = config.traffic_lights_top_offset.unwrap_or_else(|| {
            ((header_h - traffic_lights::HEIGHT) * 0.5).max(4.0)
        });
        // In Apple macOS human interface design (especially with unified/fused chrome),
        // the leading margin of the leftmost button (red close button) dynamically matches
        // its top margin (tl_x == tl_y) to achieve balanced square symmetry in the corner.
        // If an explicit leading margin is provided, that override is respected.
        let tl_x = config.traffic_lights_leading.unwrap_or(tl_y);

        let traffic_lights_hitbox = Rect::new(
            tl_x,
            tl_y,
            traffic_lights::TOTAL_WIDTH,
            traffic_lights::HEIGHT,
        );

        // Exclusion zone: covers the corner up to EXCLUSION_WIDTH or (tl_x + TOTAL_WIDTH + padding)
        let traffic_lights_exclusion_zone = Rect::new(
            0.0,
            0.0,
            traffic_lights::EXCLUSION_WIDTH.max(tl_x + traffic_lights::TOTAL_WIDTH + 8.0),
            header_h,
        );

        let header_rect = Rect::new(0.0, 0.0, width, header_h);

        match config.mode {
            ChromeLayoutMode::Separate { titlebar_height } => {
                let titlebar_h = titlebar_height.min(height);
                let content_rect = Rect::new(0.0, titlebar_h, width, (height - titlebar_h).max(0.0));

                // In separate mode, the whole titlebar except traffic lights and right margin is draggable
                let drag_start_x = traffic_lights_exclusion_zone.max_x();
                let drag_w = (width - drag_start_x - config.toolbar_action_reserved_width).max(0.0);
                let drag_rect = Rect::new(drag_start_x, 0.0, drag_w, titlebar_h);

                // Title safe area: centered between left and right margins
                let title_w = 200.0_f32.min(width * 0.4);
                let title_x = (width - title_w) * 0.5;
                let centered_title_rect = if title_x > drag_start_x {
                    Some(Rect::new(title_x, 0.0, title_w, titlebar_h))
                } else {
                    None
                };

                // Left-aligned title: starts strictly 15px to the right of traffic lights
                let left_title_start_x = traffic_lights_hitbox.max_x() + traffic_lights::TITLE_CLEARANCE;
                let left_title_w = (width - left_title_start_x - config.toolbar_action_reserved_width).max(0.0);
                let left_aligned_title_rect = if left_title_w > 10.0 {
                    Some(Rect::new(left_title_start_x, 0.0, left_title_w, titlebar_h))
                } else {
                    None
                };

                Self {
                    window_size: (width, height),
                    mode: config.mode,
                    state: config.state,
                    scale_factor: config.scale_factor,
                    rim_insets,
                    safe_client_rect,
                    traffic_lights_hitbox,
                    traffic_lights_exclusion_zone,
                    header_rect,
                    drag_regions: vec![drag_rect],
                    content_rect,
                    sidebar_rect: None,
                    sidebar_safe_content_rect: None,
                    centered_title_rect,
                    left_aligned_title_rect,
                    content_safe_insets: Insets::new(0.0, 0.0, 0.0, 0.0),
                }
            }
            ChromeLayoutMode::Unified { header_height: _, sidebar_width } => {
                let sb_w = sidebar_width.map(|w| w.min(width * 0.6));

                let (sidebar_rect, sidebar_safe_content_rect, content_rect) = if let Some(sb_w) = sb_w {
                    let sb_r = Rect::new(0.0, 0.0, sb_w, height);
                    // Inside the sidebar, content starts BELOW the traffic lights exclusion zone
                    let sb_safe = Rect::new(0.0, header_h, sb_w, (height - header_h).max(0.0));
                    // Main content takes the right pane
                    let content_r = Rect::new(sb_w, 0.0, width - sb_w, height);
                    (Some(sb_r), Some(sb_safe), content_r)
                } else {
                    (None, None, Rect::new(0.0, 0.0, width, height))
                };

                // Draggable area:
                // Across the entire top header strip, from the traffic lights exclusion zone
                // all the way to the toolbar action buttons on the far right.
                let drag_start_x = traffic_lights_exclusion_zone.max_x();
                let full_drag_w =
                    (width - drag_start_x - config.toolbar_action_reserved_width).max(0.0);
                let drag_regions = vec![Rect::new(drag_start_x, 0.0, full_drag_w, header_h)];

                Self {
                    window_size: (width, height),
                    mode: config.mode,
                    state: config.state,
                    scale_factor: config.scale_factor,
                    rim_insets,
                    safe_client_rect,
                    traffic_lights_hitbox,
                    traffic_lights_exclusion_zone,
                    header_rect,
                    drag_regions,
                    content_rect,
                    sidebar_rect,
                    sidebar_safe_content_rect,
                    centered_title_rect: None,
                    left_aligned_title_rect: None,
                    content_safe_insets: Insets::new(header_h, 0.0, 0.0, 0.0),
                }
            }
        }
    }

    /// Performs a semantic hit test at the given coordinates `(px, py)`.
    #[must_use]
    pub fn hit_test(&self, px: f32, py: f32) -> WindowHitZone {
        if px < 0.0 || px > self.window_size.0 || py < 0.0 || py > self.window_size.1 {
            return WindowHitZone::Outside;
        }

        // 0. Non-client window rim intercept:
        // Reserved strictly for window borders/resizing. Application widgets cannot occupy or intercept this.
        if px < self.rim_insets.left
            || px > (self.window_size.0 - self.rim_insets.right)
            || py < self.rim_insets.top
            || py > (self.window_size.1 - self.rim_insets.bottom)
        {
            return WindowHitZone::WindowRim;
        }

        // 1. Traffic lights takes highest priority within safe client area
        if self.traffic_lights_hitbox.contains(px, py) {
            return WindowHitZone::TrafficLights;
        }

        // 2. Draggable chrome regions
        for drag in &self.drag_regions {
            if drag.contains(px, py) {
                return WindowHitZone::DraggableChrome;
            }
        }

        // 3. Top header action area
        if self.header_rect.contains(px, py) && px > (self.window_size.0 - 180.0) {
            return WindowHitZone::ToolbarAction;
        }

        // 4. Sidebar check
        if let Some(sidebar) = self.sidebar_rect
            && sidebar.contains(px, py)
        {
            return WindowHitZone::Sidebar;
        }

        // 5. Default to content area
        WindowHitZone::Content
    }

    /// Convenience helper for point-based hit test.
    #[must_use]
    pub fn hit_test_point(&self, point: Point) -> WindowHitZone {
        self.hit_test(point.x, point.y)
    }

    /// Returns whether the given point falls inside the traffic lights exclusion zone.
    #[must_use]
    pub fn is_in_traffic_lights_exclusion(&self, px: f32, py: f32) -> bool {
        self.traffic_lights_exclusion_zone.contains(px, py)
    }

    /// Returns the resize direction if the given coordinate falls on a window rim edge or corner.
    #[must_use]
    pub fn hit_test_resize_direction(&self, px: f32, py: f32) -> Option<crate::geometry::ResizeDirection> {
        use crate::geometry::ResizeDirection;
        if self.hit_test(px, py) != WindowHitZone::WindowRim {
            return None;
        }

        let corner_slop = 12.0_f32;
        let is_left = px <= (self.rim_insets.left + corner_slop);
        let is_right = px >= (self.window_size.0 - self.rim_insets.right - corner_slop);
        let is_top = py <= (self.rim_insets.top + corner_slop);
        let is_bottom = py >= (self.window_size.1 - self.rim_insets.bottom - corner_slop);

        if is_top && is_left {
            Some(ResizeDirection::TopLeft)
        } else if is_top && is_right {
            Some(ResizeDirection::TopRight)
        } else if is_bottom && is_left {
            Some(ResizeDirection::BottomLeft)
        } else if is_bottom && is_right {
            Some(ResizeDirection::BottomRight)
        } else if is_top {
            Some(ResizeDirection::Top)
        } else if is_bottom {
            Some(ResizeDirection::Bottom)
        } else if is_left {
            Some(ResizeDirection::Left)
        } else if is_right {
            Some(ResizeDirection::Right)
        } else {
            None
        }
    }
}

/// Primitives and drawing instructions for window chrome and borders.
///
/// Enables custom rendering pipelines to draw crisp borders, separator lines,
/// and frosted sidebar extensions matching the computed chrome metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChromeDrawPlan {
    /// Whether to render a horizontal divider line beneath the titlebar.
    /// Usually `true` for `Separate` mode, `false` for `Unified` mode.
    pub show_titlebar_separator: bool,
    /// Y coordinate of the titlebar bottom separator line.
    pub titlebar_separator_y: f32,
    /// Whether to render a vertical divider line between sidebar and content.
    pub show_sidebar_separator: bool,
    /// X coordinate of the sidebar separator line.
    pub sidebar_separator_x: Option<f32>,
    /// Header background rectangle.
    pub header_rect: Rect,
}

impl ChromeDrawPlan {
    /// Generates a draw plan from computed metrics.
    #[must_use]
    pub fn from_metrics(metrics: &WindowChromeMetrics) -> Self {
        match metrics.mode {
            ChromeLayoutMode::Separate { titlebar_height } => Self {
                show_titlebar_separator: true,
                titlebar_separator_y: titlebar_height,
                show_sidebar_separator: false,
                sidebar_separator_x: None,
                header_rect: metrics.header_rect,
            },
            ChromeLayoutMode::Unified { sidebar_width, .. } => Self {
                show_titlebar_separator: false,
                titlebar_separator_y: 0.0,
                show_sidebar_separator: sidebar_width.is_some(),
                sidebar_separator_x: sidebar_width,
                header_rect: metrics.header_rect,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_separate_layout_metrics() {
        let config = WindowChromeConfig::separate(44.0);
        let metrics = WindowChromeMetrics::compute(800.0, 600.0, &config);

        assert_eq!(metrics.header_rect, Rect::new(0.0, 0.0, 800.0, 44.0));
        assert_eq!(metrics.content_rect, Rect::new(0.0, 44.0, 800.0, 556.0));
        assert!(metrics.sidebar_rect.is_none());
        assert!(metrics.traffic_lights_exclusion_zone.width >= 54.0);

        // Traffic lights hit
        assert_eq!(
            metrics.hit_test(metrics.traffic_lights_hitbox.x + 4.0, metrics.traffic_lights_hitbox.y + 4.0),
            WindowHitZone::TrafficLights
        );

        // Left-aligned title strictly 15px after traffic lights
        let title_rect = metrics.left_aligned_title_rect.expect("left-aligned title exists");
        assert_eq!(title_rect.x, metrics.traffic_lights_hitbox.max_x() + 15.0);

        // Content hit
        assert_eq!(metrics.hit_test(100.0, 100.0), WindowHitZone::Content);
    }

    #[test]
    fn test_unified_layout_metrics() {
        let config = WindowChromeConfig::unified(52.0, Some(200.0));
        let metrics = WindowChromeMetrics::compute(1000.0, 700.0, &config);

        assert_eq!(metrics.header_rect, Rect::new(0.0, 0.0, 1000.0, 52.0));
        assert_eq!(metrics.sidebar_rect, Some(Rect::new(0.0, 0.0, 200.0, 700.0)));
        assert_eq!(metrics.content_rect, Rect::new(200.0, 0.0, 800.0, 700.0));

        // In unified mode, sidebar safe area begins below header_h
        assert_eq!(
            metrics.sidebar_safe_content_rect,
            Some(Rect::new(0.0, 52.0, 200.0, 648.0))
        );

        // Sidebar hit test
        assert_eq!(metrics.hit_test(100.0, 100.0), WindowHitZone::Sidebar);
        // Main content hit test
        assert_eq!(metrics.hit_test(300.0, 100.0), WindowHitZone::Content);
    }

    #[test]
    fn test_draw_plan_generation() {
        let sep_config = WindowChromeConfig::separate(38.0);
        let sep_metrics = WindowChromeMetrics::compute(600.0, 400.0, &sep_config);
        let sep_plan = ChromeDrawPlan::from_metrics(&sep_metrics);
        assert!(sep_plan.show_titlebar_separator);
        assert_eq!(sep_plan.titlebar_separator_y, 38.0);
        assert!(!sep_plan.show_sidebar_separator);

        let uni_config = WindowChromeConfig::unified(50.0, Some(180.0));
        let uni_metrics = WindowChromeMetrics::compute(600.0, 400.0, &uni_config);
        let uni_plan = ChromeDrawPlan::from_metrics(&uni_metrics);
        assert!(!uni_plan.show_titlebar_separator);
        assert!(uni_plan.show_sidebar_separator);
        assert_eq!(uni_plan.sidebar_separator_x, Some(180.0));
    }

    #[test]
    fn test_unified_header_single_pane() {
        let config = WindowChromeConfig::unified_header(48.0);
        let metrics = WindowChromeMetrics::compute(900.0, 600.0, &config);

        assert_eq!(metrics.header_rect, Rect::new(0.0, 0.0, 900.0, 48.0));
        assert!(metrics.sidebar_rect.is_none());
        assert!(metrics.sidebar_safe_content_rect.is_none());
        // Content occupies the full window dimensions
        assert_eq!(metrics.content_rect, Rect::new(0.0, 0.0, 900.0, 600.0));
        // Drag regions span continuously across the top from traffic lights clearance
        assert!(!metrics.drag_regions.is_empty());
        let drag_rect = metrics.drag_regions[0];
        assert_eq!(drag_rect.y, 0.0);
        assert_eq!(drag_rect.height, 48.0);
        assert!(drag_rect.width > 600.0);
    }

    #[test]
    fn test_window_rim_specifications() {
        assert_eq!(window_rim::LIGHT_RIM_WIDTH, 1.0);
        assert_eq!(window_rim::DARK_OUTER_RIM_WIDTH, 1.0);
        assert_eq!(window_rim::DARK_INNER_RIM_WIDTH, 1.0);
        assert_eq!(
            window_rim::DARK_TOTAL_RIM_WIDTH,
            window_rim::DARK_OUTER_RIM_WIDTH + window_rim::DARK_INNER_RIM_WIDTH
        );
        assert_eq!(window_rim::DARK_TOTAL_RIM_WIDTH, 2.0);

        assert_eq!(window_rim::DARK_INNER_RIM_COLOR_RGB8, (70, 70, 70));
        assert_eq!(window_rim::rim_thickness(false), 1.0);
        assert_eq!(window_rim::rim_thickness(true), 2.0);
        assert_eq!(window_rim::insets(false), Insets::uniform(1.0));
        assert_eq!(window_rim::insets(true), Insets::uniform(2.0));
    }

    #[test]
    fn test_window_rim_safe_client_area_and_hit_test() {
        let config = WindowChromeConfig::separate(32.0);

        // Dark mode: 2px compound rim
        let dark_metrics = WindowChromeMetrics::compute_with_theme(800.0, 600.0, &config, true);
        assert_eq!(dark_metrics.rim_insets, Insets::uniform(2.0));
        assert_eq!(dark_metrics.safe_client_rect, Rect::new(2.0, 2.0, 796.0, 596.0));

        // Light mode: 1px subtle rim
        let light_metrics = WindowChromeMetrics::compute_with_theme(800.0, 600.0, &config, false);
        assert_eq!(light_metrics.rim_insets, Insets::uniform(1.0));
        assert_eq!(light_metrics.safe_client_rect, Rect::new(1.0, 1.0, 798.0, 598.0));

        // Hit testing on the rim:
        // Top-left boundary
        assert_eq!(dark_metrics.hit_test(0.5, 0.5), WindowHitZone::WindowRim);
        assert_eq!(dark_metrics.hit_test(1.5, 1.5), WindowHitZone::WindowRim);
        // Light mode at (1.5, 1.5) is inside the client area
        assert_eq!(light_metrics.hit_test(0.5, 0.5), WindowHitZone::WindowRim);
        assert_ne!(light_metrics.hit_test(1.5, 1.5), WindowHitZone::WindowRim);

        // Right and bottom edges
        assert_eq!(dark_metrics.hit_test(799.0, 300.0), WindowHitZone::WindowRim);
        assert_eq!(dark_metrics.hit_test(400.0, 599.0), WindowHitZone::WindowRim);

        // Outside window bounds
        assert_eq!(dark_metrics.hit_test(-1.0, 10.0), WindowHitZone::Outside);
        assert_eq!(dark_metrics.hit_test(801.0, 10.0), WindowHitZone::Outside);
    }

    #[test]
    fn test_fullscreen_rim_collapse() {
        let config = WindowChromeConfig::unified_header(40.0)
            .with_state(WindowState::Fullscreen);
        let dark_metrics = WindowChromeMetrics::compute_with_theme(1920.0, 1080.0, &config, true);

        // Fullscreen should collapse rim insets to ZERO
        assert_eq!(dark_metrics.rim_insets, Insets::ZERO);
        assert_eq!(dark_metrics.safe_client_rect, Rect::new(0.0, 0.0, 1920.0, 1080.0));
    }

    #[test]
    fn test_hit_test_resize_direction() {
        use crate::geometry::ResizeDirection;
        let config = WindowChromeConfig::separate(32.0);
        let metrics = WindowChromeMetrics::compute_with_theme(800.0, 600.0, &config, true);

        // Top-left corner
        assert_eq!(metrics.hit_test_resize_direction(0.5, 0.5), Some(ResizeDirection::TopLeft));
        // Top edge
        assert_eq!(metrics.hit_test_resize_direction(400.0, 0.5), Some(ResizeDirection::Top));
        // Bottom-right corner
        assert_eq!(metrics.hit_test_resize_direction(799.0, 599.0), Some(ResizeDirection::BottomRight));
        // Right edge
        assert_eq!(metrics.hit_test_resize_direction(799.5, 300.0), Some(ResizeDirection::Right));
        // Inside window content returns None
        assert_eq!(metrics.hit_test_resize_direction(400.0, 300.0), None);
    }

    #[test]
    fn test_traffic_lights_dynamic_symmetric_margin() {
        // 1. Unified 52.0px chrome:
        // top margin = (52.0 - 14.0) * 0.5 = 19.0px.
        // Dynamic leading margin must match top margin: tl_x == tl_y == 19.0px.
        let config_52 = WindowChromeConfig::unified_header(52.0);
        let metrics_52 = WindowChromeMetrics::compute(800.0, 600.0, &config_52);
        assert_eq!(metrics_52.traffic_lights_hitbox.y, 19.0);
        assert_eq!(metrics_52.traffic_lights_hitbox.x, 19.0);
        assert_eq!(metrics_52.traffic_lights_hitbox.width, 60.0);
        assert_eq!(metrics_52.traffic_lights_hitbox.height, 14.0);

        // 2. Separate 32.0px titlebar:
        // top margin = (32.0 - 14.0) * 0.5 = 9.0px.
        // Dynamic leading margin: tl_x == tl_y == 9.0px.
        let config_32 = WindowChromeConfig::separate(32.0);
        let metrics_32 = WindowChromeMetrics::compute(800.0, 600.0, &config_32);
        assert_eq!(metrics_32.traffic_lights_hitbox.y, 9.0);
        assert_eq!(metrics_32.traffic_lights_hitbox.x, 9.0);

        // 3. Explicit override:
        let config_custom = WindowChromeConfig::unified_header(52.0)
            .with_traffic_lights_leading(10.0);
        let metrics_custom = WindowChromeMetrics::compute(800.0, 600.0, &config_custom);
        assert_eq!(metrics_custom.traffic_lights_hitbox.y, 19.0);
        assert_eq!(metrics_custom.traffic_lights_hitbox.x, 10.0);
    }
}
