//! Traffic lights.
//!
//! The implementation is the standalone [`bmol_window_traffic_lights`] crate.
//! This module only preserves the historically public
//! `bmol_window_shell::traffic_lights` path so existing applications keep
//! compiling; new code should depend on the crate directly.

pub use bmol_window_traffic_lights::layout::*;
pub use bmol_window_traffic_lights::*;
