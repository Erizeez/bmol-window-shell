//! Window frame chrome layout, collision detection, and layout metrics.
//!
//! Provides two standard window chrome layouts:
//! 1. `ChromeLayoutMode::Separate`: Standalone titlebar separated from content below.
//! 2. `ChromeLayoutMode::Unified`: Integrated chrome where content / sidebar extends to the top edge.

use crate::geometry::{Insets, Point, Rect};

/// Default measurements for macOS-style traffic lights.
pub mod traffic_lights {
    pub const DIAMETER: f32 = 12.0;
    pub const SPACING: f32 = 8.0;
    pub const LEADING_MARGIN: f32 = 18.0;
    /// Total width across the three lights (12 + 8 + 12 + 8 + 12 = 52).
    pub const TOTAL_WIDTH: f32 = 52.0;
    pub const HEIGHT: f32 = 12.0;
    /// Recommended horizontal clearance width including padding.
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


/// The layout mode for the window chrome / titlebar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChromeLayoutMode {
    /// Separate titlebar mode: The titlebar sits strictly above the content.
    /// Content starts at `y = titlebar_height`.
    Separate {
        /// Height of the standalone titlebar in logical points (typically 36.0 - 52.0).
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
        Self::Separate { titlebar_height: 32.0 }
    }

    /// Unified chrome with standard header height (52.0 pt) and optional sidebar width (220.0 pt).
    #[must_use]
    pub const fn unified_default(sidebar_width: Option<f32>) -> Self {
        Self::Unified {
            header_height: 52.0,
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

/// Configuration parameters for window chrome calculation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowChromeConfig {
    pub mode: ChromeLayoutMode,
    /// Distance from the window left edge to the traffic lights.
    pub traffic_lights_leading: f32,
    /// Custom vertical offset for traffic lights, or None for automatic vertical centering.
    pub traffic_lights_top_offset: Option<f32>,
    /// Minimum width reserved for title text or toolbar actions.
    pub toolbar_action_reserved_width: f32,
}

impl Default for WindowChromeConfig {
    fn default() -> Self {
        Self {
            mode: ChromeLayoutMode::unified_default(Some(220.0)),
            traffic_lights_leading: traffic_lights::LEADING_MARGIN,
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
            traffic_lights_leading: traffic_lights::LEADING_MARGIN,
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
            traffic_lights_leading: traffic_lights::LEADING_MARGIN,
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
}

/// Identifies which semantic zone a given coordinate hits within the window chrome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowHitZone {
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
/// 1. Avoid overlapping interactive widgets with the traffic lights exclusion zone.
/// 2. Implement native-style window dragging without intercepting inner widget clicks.
/// 3. Size and position content views, sidebars, and titlebar separators correctly.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowChromeMetrics {
    /// Full window dimensions (width, height).
    pub window_size: (f32, f32),
    /// Active layout mode.
    pub mode: ChromeLayoutMode,

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
    #[must_use]
    pub fn compute(window_width: f32, window_height: f32, config: &WindowChromeConfig) -> Self {
        let width = window_width.max(1.0);
        let height = window_height.max(1.0);

        let header_h = config.mode.header_height().min(height);

        // Traffic lights positioning
        let tl_y = config.traffic_lights_top_offset.unwrap_or_else(|| {
            ((header_h - traffic_lights::HEIGHT) * 0.5).max(4.0)
        });
        let traffic_lights_hitbox = Rect::new(
            config.traffic_lights_leading,
            tl_y,
            traffic_lights::TOTAL_WIDTH,
            traffic_lights::HEIGHT,
        );

        // Exclusion zone: covers the corner up to EXCLUSION_WIDTH
        let traffic_lights_exclusion_zone = Rect::new(
            0.0,
            0.0,
            traffic_lights::EXCLUSION_WIDTH.max(config.traffic_lights_leading + traffic_lights::TOTAL_WIDTH + 8.0),
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

                // Left-aligned title: starts strictly 16px to the right of traffic lights
                let left_title_start_x = traffic_lights_hitbox.max_x() + 16.0;
                let left_title_w = (width - left_title_start_x - config.toolbar_action_reserved_width).max(0.0);
                let left_aligned_title_rect = if left_title_w > 10.0 {
                    Some(Rect::new(left_title_start_x, 0.0, left_title_w, titlebar_h))
                } else {
                    None
                };

                Self {
                    window_size: (width, height),
                    mode: config.mode,
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

        // 1. Traffic lights takes highest priority
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
        assert!(metrics.traffic_lights_exclusion_zone.width >= 52.0);

        // Traffic lights hit
        assert_eq!(
            metrics.hit_test(traffic_lights::LEADING_MARGIN + 4.0, 22.0),
            WindowHitZone::TrafficLights
        );

        // Left-aligned title strictly 16px after traffic lights
        let title_rect = metrics.left_aligned_title_rect.expect("left-aligned title exists");
        assert_eq!(title_rect.x, metrics.traffic_lights_hitbox.max_x() + 16.0);

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
}
