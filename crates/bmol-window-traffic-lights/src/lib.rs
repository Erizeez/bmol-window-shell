//! Authentic macOS window controls (traffic lights) for `bmol-window-shell`.
//!
//! This crate is the single owner of the traffic-light feature. It holds, in
//! dependency order:
//!
//! - [`action`] — the semantic action of each control, and the green control's
//!   expand behaviour.
//! - [`palette`] — Apple colourimetry and the extracted vector glyph sources.
//! - [`layout`] — measured geometry (14 pt diameter, 9 pt spacing) and the
//!   sample-group layout used by the standalone laboratory.
//! - [`state`] — the interaction state machine: group hover, per-control press
//!   tint, and the press scale spring. This module never mentions Iced.
//! - [`tuning`] — the physical-glass material knobs.
//! - [`interaction`] — the shared interaction frame the compositor reads, so
//!   the glyph layer and the GPU material can never disagree about a value.
//! - [`scene`] — the renderer-agnostic [`liquid_glass_scene::GlassScene`]
//!   builder for the spheres.
//! - [`widget`] — the Iced glyph layer and hit testing (feature `iced`).
//!
//! # Architecture
//!
//! Exactly one implementation owns each concern:
//!
//! 1. [`TrafficLightsState`] owns *interaction* (hover, press, scale).
//! 2. [`interaction`] publishes those values once per tick.
//! 3. [`scene`] turns them into a physical-glass [`liquid_glass_scene::GlassScene`].
//! 4. [`widget`] draws only the Apple vector glyphs and reports pointer facts.
//!
//! The Liquid Glass *effect* itself — the `TrafficLightPhysical` material
//! variant, its style, and the shader that implements it — lives in liquid-rs.
//! This crate never reimplements it; it only selects and tunes it.
//!
//! # Press scale contract
//!
//! A control rests at [`PRESS_SCALE_REST`] and is held at [`PRESS_SCALE_PEAK`]
//! while the pointer is down. Dragging off the control clears the tint but
//! **keeps the scale enlarged until the mouse button is released**; releasing
//! always returns the control to [`PRESS_SCALE_REST`].

// Pedantic lints the rest of this workspace also tolerates. The traffic-light
// palette and geometry are deliberately expressed as calibrated float literals
// and boolean state flags, and the public surface mirrors Apple's vocabulary
// (AppKit, macOS) which `doc_markdown` would otherwise flag.
#![allow(
    clippy::doc_markdown,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::fn_params_excessive_bools,
    clippy::struct_excessive_bools,
    clippy::must_use_candidate,
    clippy::if_not_else,
    clippy::too_many_lines
)]

pub mod action;
pub mod interaction;
pub mod layout;
pub mod palette;
pub mod scene;
pub mod state;
pub mod tuning;

#[cfg(feature = "iced")]
pub mod widget;

pub use action::{ControlAction, WindowControlAction, WindowExpandBehavior};

pub use layout::{
    GROUP_COUNT, GROUP_LEN, WINDOW_CONTROL_DISABLED_IDS, WINDOW_CONTROL_DISABLED_X,
    WINDOW_CONTROL_DISABLED_Y, WINDOW_CONTROL_GAP, WINDOW_CONTROL_GROUPS,
    WINDOW_CONTROL_INACTIVE_IDS, WINDOW_CONTROL_INACTIVE_X, WINDOW_CONTROL_INACTIVE_Y,
    WINDOW_CONTROL_LARGE_GAP, WINDOW_CONTROL_LARGE_IDS, WINDOW_CONTROL_LARGE_SIZE,
    WINDOW_CONTROL_LARGE_X, WINDOW_CONTROL_LARGE_Y, WINDOW_CONTROL_NATIVE_IDS,
    WINDOW_CONTROL_NATIVE_SIZE, WINDOW_CONTROL_NATIVE_X, WINDOW_CONTROL_NATIVE_Y,
    WINDOW_CONTROL_RAW_GROUPS, WINDOW_CONTROL_REFERENCE_IDS, WINDOW_CONTROL_REFERENCE_RAW_IDS,
    WINDOW_CONTROL_REFERENCE_X, WINDOW_CONTROL_REFERENCE_Y, control_hover_slop,
};

pub use palette::{
    SVG_CLOSE, SVG_MAXIMIZE, SVG_MINIMIZE, SVG_SPECULAR_ARC, SVG_ZOOM, TrafficLightSpherePalette,
    blend_color, blend_rgba, blend_sphere_palette, resolve_active_button_colors, resolve_active_sphere_palette,
    resolve_button_colors, resolve_inactive_button_colors, resolve_inactive_sphere_palette, rgb8,
    traffic_light_source_colors, window_control_glyph_color, window_control_glyph_size,
    window_control_icon,
};

pub use state::{
    INTERACTION_ENTER_ANIMATION_TIME_CONSTANT, INTERACTION_EXIT_ANIMATION_TIME_CONSTANT,
    PRESS_SCALE_PEAK, PRESS_SCALE_REST, PRESS_SPRING_BOUNCE, PRESS_SPRING_DURATION,
    TrafficLightSlot, TrafficLightsEvent, TrafficLightsState,
};

pub use tuning::{TrafficLightMaterialFields, WindowControlTuning};

pub use scene::{
    group_hover, push_traffic_light_group, scale_scene, sphere_bounds, traffic_light_material,
    traffic_light_source_color, traffic_lights_scene_for_viewport,
};

pub use interaction::{
    SLOT_COUNT, TrafficLightInteraction, TrafficLightsFrame, active_window_control_origin,
    active_window_control_tuning, all_window_control_ids, group_index_for_raw_id, publish_group,
    publish_groups, reset_groups, set_window_control_origin, set_window_control_tuning,
    slot_index_for_raw_id, snapshot,
    window_control_group_ids, window_control_group_index, window_control_group_progress,
    window_control_interaction, window_control_press_progress, window_control_scale,
    window_control_slot_index,
};

#[cfg(feature = "iced")]
pub use widget::{
    CircleStyle, ControlGroup, ControlGroupStyle, TrafficLightGroup, centered, control_group,
    glass_passthrough, is_document_edited, native_group_style, positioned_control_group,
    set_document_edited, set_glass_passthrough, view_single_button,
    view_single_button_interactive, view_traffic_lights_all_inclusive, window_control_circle,
    window_control_group, window_control_status_dot,
};
