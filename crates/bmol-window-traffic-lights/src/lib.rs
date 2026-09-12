//! Authentic macOS window controls (traffic lights) for `bmol-window-shell`.
//!
//! This crate owns the traffic-light **semantics**: what each control means,
//! how hover and press animate, how the material is tuned, and how a frame of
//! that state becomes GPU glass.
//!
//! The **drawing** — the Apple vector glyphs, the optical hit testing, and the
//! overlay routing that keeps the glyphs above the glass — lives in the shared
//! widget layer, [`liquid_glass_ui::traffic_light`], because an application
//! needs the same widget a window shell does.
//!
//! # Where each concern lives
//!
//! | Concern | Home |
//! |---------|------|
//! | Semantic actions and the green control's expand behaviour | [`action`] (widget layer) |
//! | Measured metrics and the laboratory sample layout | [`layout`] (widget layer) |
//! | Apple colourimetry and the extracted vector glyph sources | [`palette`] (widget layer) |
//! | The pointer-event vocabulary the widget emits | [`event`] (widget layer) |
//! | The Iced glyph layer and hit testing | [`liquid_glass_ui::traffic_light::widget`] |
//! | Interaction state machine (hover, press, press-scale spring) | [`state`] |
//! | The shared interaction frame the compositor reads | [`interaction`] |
//! | Material tuning | [`tuning`] |
//! | Renderer-agnostic [`liquid_glass_scene::GlassScene`] construction | [`scene`] |
//! | The state machine → widget bridge | [`widget`] (feature `iced`) |
//!
//! ## Why the control data is not owned here
//!
//! The widget needs the actions, the metrics, the colourimetry and the glyph
//! sources, so they are owned by the widget layer and re-exported below.
//! Keeping a second copy here is what let the two drift apart: the shell's
//! glyphs once disagreed with the widget's about the same control.
//!
//! # Architecture
//!
//! Exactly one implementation owns each concern:
//!
//! 1. [`TrafficLightsState`] owns *interaction* (hover, press, scale).
//! 2. [`interaction`] publishes those values once per tick.
//! 3. [`scene`] turns them into a physical-glass [`liquid_glass_scene::GlassScene`].
//! 4. The widget layer draws the Apple vector glyphs and reports pointer facts.
//! 5. [`widget`] bridges 1–4 for a real window.
//!
//! The Liquid Glass *effect* itself — the bead material variant, its style, and
//! the shader that implements it — lives in liquid-rs. This crate never
//! reimplements it; it only selects and tunes it.
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

// Re-exported as modules rather than only as items: the widget layer owns
// these, and keeping the module paths alive means the crate still presents one
// public surface instead of forwarding a bare list of names.
pub use liquid_glass_ui::traffic_light::{action, event, layout, palette};

pub mod interaction;
pub mod scene;
pub mod state;
pub mod tuning;

#[cfg(feature = "iced")]
pub mod widget;

pub use action::{ControlAction, WindowControlAction, WindowExpandBehavior};

pub use event::TrafficLightsEvent;

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
    TrafficLightSlot, TrafficLightsState,
};

pub use tuning::{TrafficLightMaterialFields, WindowControlTuning};

pub use scene::{
    group_hover, push_traffic_light_group, scale_scene, sphere_bounds, traffic_light_material,
    traffic_light_source_color, traffic_lights_scene_for_viewport,
};

pub use interaction::{
    SLOT_COUNT, TrafficLightInteraction, TrafficLightsFrame, active_window_control_origin,
    active_window_control_tuning, all_window_control_ids, group_index_for_raw_id, publish_group,
    publish_groups, reset_groups, set_window_control_group_hover,
    set_window_control_group_progress, set_window_control_origin,
    set_window_control_press_progress, set_window_control_scale, set_window_control_tuning,
    slot_index_for_raw_id, snapshot, window_control_group_hover_target, window_control_group_ids,
    window_control_group_index, window_control_group_progress, window_control_interaction,
    window_control_press_progress, window_control_scale, window_control_slot_index,
};

// The widget and its builders, owned by the widget layer.
#[cfg(feature = "iced")]
pub use liquid_glass_ui::traffic_light::{
    CircleStyle, ControlGroup, ControlGroupStyle, TrafficLightGroup, centered, control_group,
    glass_passthrough, is_document_edited, positioned_control_group, set_document_edited,
    set_glass_passthrough, view_single_button, view_single_button_interactive,
    window_control_circle, window_control_group, window_control_status_dot,
};

// The shell-side bridge: the measured-origin publisher and the state-machine
// adapters.
#[cfg(feature = "iced")]
pub use widget::{native_group_style, view_traffic_lights_all_inclusive};
