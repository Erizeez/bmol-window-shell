//! Traffic lights.
//!
//! The implementation is the standalone [`bmol_window_traffic_lights`] crate.
//! This module only preserves the historically public
//! `bmol_window_shell::traffic_lights` path so existing applications keep
//! compiling; new code should depend on the crate directly.
//!
//! Note where the pieces come from: the semantic actions, the measured metrics,
//! the Apple colourimetry, the glyph sources, and the Iced widget are owned by
//! the shared widget layer (`liquid-glass-ui`) and re-exported through
//! `bmol_window_traffic_lights`; the interaction state machine, the shared
//! interaction frame, the material tuning, and the `GlassScene` builder are
//! owned by that crate itself. Both globs below are deliberate — this path is
//! the compatibility surface, so it re-exports what either side offers.

pub use bmol_window_traffic_lights::layout::*;
pub use bmol_window_traffic_lights::*;
