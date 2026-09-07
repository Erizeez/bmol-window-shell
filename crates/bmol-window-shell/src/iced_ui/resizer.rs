//! Border resizing and cursor mapping for frameless windows.
//!
//! Bridges platform [`ResizeDirection`] to Iced window resize commands and
//! mouse cursor interactions.

use iced::mouse::Interaction;
use iced::window::Direction;

use crate::platform::{ResizeDirection, WindowChromeMetrics};

/// Converts a platform [`ResizeDirection`] to Iced's window [`Direction`].
#[must_use]
pub const fn resize_direction_to_window_direction(dir: ResizeDirection) -> Direction {
    match dir {
        ResizeDirection::Top => Direction::North,
        ResizeDirection::Bottom => Direction::South,
        ResizeDirection::Left => Direction::West,
        ResizeDirection::Right => Direction::East,
        ResizeDirection::TopLeft => Direction::NorthWest,
        ResizeDirection::TopRight => Direction::NorthEast,
        ResizeDirection::BottomLeft => Direction::SouthWest,
        ResizeDirection::BottomRight => Direction::SouthEast,
    }
}

/// Converts a platform [`ResizeDirection`] to the appropriate mouse [`Interaction`] cursor.
#[must_use]
pub const fn resize_direction_to_interaction(dir: ResizeDirection) -> Interaction {
    match dir {
        ResizeDirection::Top | ResizeDirection::Bottom => Interaction::ResizingVertically,
        ResizeDirection::Left | ResizeDirection::Right => Interaction::ResizingHorizontally,
        ResizeDirection::TopLeft | ResizeDirection::BottomRight => {
            Interaction::ResizingDiagonallyDown
        }
        ResizeDirection::TopRight | ResizeDirection::BottomLeft => {
            Interaction::ResizingDiagonallyUp
        }
    }
}

/// Resolves the resize direction, window resize target, and mouse interaction at coordinate (px, py).
///
/// Returns `None` if the cursor is not currently hovering over a resizable window rim or corner.
#[must_use]
pub fn resolve_resize_at(
    metrics: &WindowChromeMetrics,
    px: f32,
    py: f32,
) -> Option<(ResizeDirection, Direction, Interaction)> {
    metrics.hit_test_resize_direction(px, py).map(|dir| {
        let win_dir = resize_direction_to_window_direction(dir);
        let cursor = resize_direction_to_interaction(dir);
        (dir, win_dir, cursor)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{WindowChromeConfig, WindowState};

    #[test]
    fn test_resize_direction_conversions() {
        assert!(matches!(
            resize_direction_to_window_direction(ResizeDirection::Top),
            Direction::North
        ));
        assert!(matches!(
            resize_direction_to_window_direction(ResizeDirection::Bottom),
            Direction::South
        ));
        assert!(matches!(
            resize_direction_to_window_direction(ResizeDirection::Left),
            Direction::West
        ));
        assert!(matches!(
            resize_direction_to_window_direction(ResizeDirection::Right),
            Direction::East
        ));
        assert!(matches!(
            resize_direction_to_window_direction(ResizeDirection::TopLeft),
            Direction::NorthWest
        ));
        assert!(matches!(
            resize_direction_to_window_direction(ResizeDirection::TopRight),
            Direction::NorthEast
        ));
        assert!(matches!(
            resize_direction_to_window_direction(ResizeDirection::BottomLeft),
            Direction::SouthWest
        ));
        assert!(matches!(
            resize_direction_to_window_direction(ResizeDirection::BottomRight),
            Direction::SouthEast
        ));

        assert_eq!(
            resize_direction_to_interaction(ResizeDirection::Top),
            Interaction::ResizingVertically
        );
        assert_eq!(
            resize_direction_to_interaction(ResizeDirection::Left),
            Interaction::ResizingHorizontally
        );
        assert_eq!(
            resize_direction_to_interaction(ResizeDirection::TopLeft),
            Interaction::ResizingDiagonallyDown
        );
        assert_eq!(
            resize_direction_to_interaction(ResizeDirection::TopRight),
            Interaction::ResizingDiagonallyUp
        );
    }

    #[test]
    fn test_resolve_resize_at() {
        let config = WindowChromeConfig::separate(32.0).with_state(WindowState::Normal);
        let metrics = WindowChromeMetrics::compute_with_theme(800.0, 600.0, &config, true);

        // Top-left corner
        let result = resolve_resize_at(&metrics, 0.5, 0.5);
        assert!(matches!(
            result,
            Some((
                ResizeDirection::TopLeft,
                Direction::NorthWest,
                Interaction::ResizingDiagonallyDown
            ))
        ));

        // Center content area
        assert!(resolve_resize_at(&metrics, 400.0, 300.0).is_none());
    }
}
