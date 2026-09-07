//! Framework-neutral geometric primitives for window layout and collision detection.

use std::fmt;

/// A 2D point in logical coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A 2D dimension in logical coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// Axis-aligned bounding box (AABB) in logical pixel space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    /// Creates a new rectangle.
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Creates a rectangle from top-left point and size.
    #[must_use]
    pub const fn from_point_size(point: Point, size: Size) -> Self {
        Self {
            x: point.x,
            y: point.y,
            width: size.width,
            height: size.height,
        }
    }

    /// The minimum X coordinate (left edge).
    #[must_use]
    pub const fn min_x(self) -> f32 {
        self.x
    }

    /// The maximum X coordinate (right edge).
    #[must_use]
    pub fn max_x(self) -> f32 {
        self.x + self.width
    }

    /// The minimum Y coordinate (top edge).
    #[must_use]
    pub const fn min_y(self) -> f32 {
        self.y
    }

    /// The maximum Y coordinate (bottom edge).
    #[must_use]
    pub fn max_y(self) -> f32 {
        self.y + self.height
    }

    /// Center point of the rectangle.
    #[must_use]
    pub fn center(self) -> Point {
        Point::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    /// Returns whether this rectangle contains the given point.
    #[must_use]
    pub fn contains(self, px: f32, py: f32) -> bool {
        px >= self.x && px <= (self.x + self.width) && py >= self.y && py <= (self.y + self.height)
    }

    /// Returns whether this rectangle contains the given Point.
    #[must_use]
    pub fn contains_point(self, point: Point) -> bool {
        self.contains(point.x, point.y)
    }

    /// Returns whether two rectangles intersect.
    #[must_use]
    pub fn intersects(self, other: &Self) -> bool {
        self.min_x() < other.max_x()
            && self.max_x() > other.min_x()
            && self.min_y() < other.max_y()
            && self.max_y() > other.min_y()
    }

    /// Insets the rectangle by the specified margins (top, right, bottom, left).
    #[must_use]
    pub fn insets(self, insets: Insets) -> Self {
        let x = self.x + insets.left;
        let y = self.y + insets.top;
        let width = (self.width - insets.left - insets.right).max(0.0);
        let height = (self.height - insets.top - insets.bottom).max(0.0);
        Self { x, y, width, height }
    }

    /// Expands the rectangle by uniform padding on all sides.
    #[must_use]
    pub fn expand(self, padding: f32) -> Self {
        Self {
            x: self.x - padding,
            y: self.y - padding,
            width: self.width + padding * 2.0,
            height: self.height + padding * 2.0,
        }
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rect(x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1})", self.x, self.y, self.width, self.height)
    }
}

/// Insets from edges (top, right, bottom, left).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Insets {
    #[must_use]
    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self { top, right, bottom, left }
    }

    #[must_use]
    pub const fn uniform(all: f32) -> Self {
        Self { top: all, right: all, bottom: all, left: all }
    }

    #[must_use]
    pub const fn symmetrical(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_contains() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert!(r.contains(10.0, 20.0));
        assert!(r.contains(50.0, 40.0));
        assert!(r.contains(110.0, 70.0));
        assert!(!r.contains(9.0, 20.0));
        assert!(!r.contains(50.0, 71.0));
    }

    #[test]
    fn test_rect_insets() {
        let r = Rect::new(0.0, 0.0, 200.0, 100.0);
        let insets = Insets::new(10.0, 20.0, 15.0, 5.0);
        let inner = r.insets(insets);
        assert_eq!(inner, Rect::new(5.0, 10.0, 175.0, 75.0));
    }

    #[test]
    fn test_rect_intersects() {
        let a = Rect::new(0.0, 0.0, 50.0, 50.0);
        let b = Rect::new(25.0, 25.0, 50.0, 50.0);
        let c = Rect::new(60.0, 60.0, 10.0, 10.0);
        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
    }
}
