//! GPU physical-glass macOS traffic lights for `bmol-window-shell`.
//!
//! This crate owns the single authentic traffic-light *style*: the scene
//! builder, the physical-glass material tuning, the calibrated per-state
//! colours, and the interaction state consumed by a Liquid Glass compositor.
//! It is renderer-agnostic (it only produces a `GlassScene`), so any Iced
//! Liquid Glass compositor can composite the same spheres.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};

use liquid_glass_scene::{
    Color, GlassId, GlassInteraction, GlassMaterial, GlassNode, GlassScene, GlassShape,
    GlassVariant, Rect, TrafficLightStyle,
};

pub const WINDOW_CONTROL_NATIVE_IDS: [GlassId; 3] = [GlassId(100), GlassId(101), GlassId(102)];
pub const WINDOW_CONTROL_REFERENCE_IDS: [GlassId; 3] = [GlassId(110), GlassId(111), GlassId(112)];
pub const WINDOW_CONTROL_LARGE_IDS: [GlassId; 3] = [GlassId(120), GlassId(121), GlassId(122)];
pub const WINDOW_CONTROL_INACTIVE_IDS: [GlassId; 3] = [GlassId(130), GlassId(131), GlassId(132)];
pub const WINDOW_CONTROL_DISABLED_IDS: [GlassId; 3] = [GlassId(140), GlassId(141), GlassId(142)];

pub const WINDOW_CONTROL_NATIVE_X: f32 = bmol_designs::traffic_lights::LEADING_MARGIN;
pub const WINDOW_CONTROL_NATIVE_Y: f32 = bmol_designs::traffic_lights::LEADING_MARGIN;
pub const WINDOW_CONTROL_REFERENCE_X: f32 = 240.0;
pub const WINDOW_CONTROL_REFERENCE_Y: f32 = 196.0;
pub const WINDOW_CONTROL_LARGE_X: f32 = 240.0;
pub const WINDOW_CONTROL_LARGE_Y: f32 = 292.0;
pub const WINDOW_CONTROL_INACTIVE_X: f32 = 240.0;
pub const WINDOW_CONTROL_INACTIVE_Y: f32 = 418.0;
pub const WINDOW_CONTROL_DISABLED_X: f32 = 240.0;
pub const WINDOW_CONTROL_DISABLED_Y: f32 = 544.0;
/// AppKit's standard traffic-light circle measures 28 px on a 2x display.
/// Keep the cross-platform sample in logical points so its 1:1 reference is
/// independent of the backing scale factor.
pub const WINDOW_CONTROL_NATIVE_SIZE: f32 = bmol_designs::traffic_lights::DIAMETER;
pub const WINDOW_CONTROL_LARGE_SIZE: f32 = 64.0;
pub const WINDOW_CONTROL_GAP: f32 = bmol_designs::traffic_lights::SPACING;
pub const WINDOW_CONTROL_LARGE_GAP: f32 = 12.0;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowControlTuning {
    pub blur_radius: f32,
    pub opacity: f32,
    pub substrate_coverage: f32,
    pub lower_substrate_coverage: f32,
    pub lower_tint_coverage: f32,
    pub angular_light: f32,
    pub light_angle: f32,
    pub light_softness: f32,
    pub body_thickness: f32,
    pub internal_scattering: f32,
    pub side_edge_darkness: f32,
    pub side_edge_width: f32,
    pub edge_side_bias: f32,
    pub edge_side_angle: f32,
    pub refraction_strength: f32,
    pub fresnel_strength: f32,
}

impl WindowControlTuning {
    pub const fn new() -> Self {
        Self {
            blur_radius: 17.0,
            opacity: 1.0,
            substrate_coverage: 0.88,
            lower_substrate_coverage: 0.54,
            lower_tint_coverage: 0.66,
            angular_light: 0.055,
            light_angle: 0.52,
            light_softness: 1.0,
            body_thickness: 0.79,
            internal_scattering: 1.0,
            side_edge_darkness: 4.0,
            side_edge_width: 1.26,
            edge_side_bias: 1.0,
            edge_side_angle: 23.0,
            refraction_strength: 0.5,
            fresnel_strength: 0.0,
        }
    }

    /// Returns the scheme-specific material preset used by the standalone
    /// controls laboratory. The dark preset intentionally removes the side
    /// absorption and refraction response while increasing Fresnel, matching
    /// the measured dark-mode control treatment.
    #[allow(dead_code)]
    #[must_use]
    pub const fn for_scheme(is_dark: bool) -> Self {
        if is_dark {
            Self {
                blur_radius: 17.0,
                internal_scattering: 1.0,
                side_edge_darkness: 0.0,
                side_edge_width: 0.5,
                opacity: 1.0,
                substrate_coverage: 0.88,
                lower_substrate_coverage: 0.54,
                lower_tint_coverage: 0.66,
                angular_light: 0.055,
                light_angle: 0.52,
                light_softness: 1.0,
                body_thickness: 0.79,
                edge_side_bias: 1.0,
                edge_side_angle: 23.0,
                refraction_strength: 0.0,
                fresnel_strength: 0.39,
            }
        } else {
            Self::new()
        }
    }

    #[must_use]
    fn clamped(self) -> Self {
        Self {
            blur_radius: self.blur_radius.clamp(0.0, 80.0),
            opacity: self.opacity.clamp(0.0, 1.0),
            substrate_coverage: self.substrate_coverage.clamp(0.0, 1.0),
            lower_substrate_coverage: self.lower_substrate_coverage.clamp(0.0, 1.0),
            lower_tint_coverage: self.lower_tint_coverage.clamp(0.0, 1.0),
            angular_light: self.angular_light.clamp(0.0, 2.0),
            light_angle: self.light_angle.clamp(0.0, 1.0),
            light_softness: self.light_softness.clamp(0.0, 1.0),
            body_thickness: self.body_thickness.clamp(0.25, 3.0),
            internal_scattering: self.internal_scattering.clamp(0.0, 1.0),
            side_edge_darkness: self.side_edge_darkness.clamp(0.0, 4.0),
            side_edge_width: self.side_edge_width.clamp(0.25, 4.0),
            edge_side_bias: self.edge_side_bias.clamp(0.0, 1.0),
            edge_side_angle: self.edge_side_angle.clamp(10.0, 80.0),
            refraction_strength: self.refraction_strength.clamp(0.0, 1.0),
            fresnel_strength: self.fresnel_strength.clamp(0.0, 1.0),
        }
    }
}

impl Default for WindowControlTuning {
    fn default() -> Self {
        Self::new()
    }
}
static ACTIVE_WINDOW_CONTROL_HOVER: AtomicU8 = AtomicU8::new(0);
// The Iced layer advances this shared value on its animation tick. The
// compositor consumes the same value for the material transition, avoiding a
// separate renderer clock that could make the glyph and colour drift by a few
// frames.
static ACTIVE_WINDOW_CONTROL_PROGRESS: Mutex<[f32; 5]> = Mutex::new([0.0; 5]);
static ACTIVE_WINDOW_CONTROL_PRESS_PROGRESS: Mutex<[f32; 15]> = Mutex::new([0.0; 15]);
static ACTIVE_WINDOW_CONTROL_SCALE: Mutex<[f32; 15]> = Mutex::new([1.0; 15]);
static ACTIVE_WINDOW_CONTROL_TUNING: Mutex<WindowControlTuning> =
    Mutex::new(WindowControlTuning::for_scheme(true));
static ACTIVE_WINDOW_CONTROL_ORIGIN: Mutex<(f32, f32)> =
    Mutex::new((WINDOW_CONTROL_NATIVE_X, WINDOW_CONTROL_NATIVE_Y));
pub fn set_window_control_origin(x: f32, y: f32) {
    if let Ok(mut origin) = ACTIVE_WINDOW_CONTROL_ORIGIN.lock() {
        *origin = (x, y);
    }
}

pub fn active_window_control_origin() -> (f32, f32) {
    ACTIVE_WINDOW_CONTROL_ORIGIN
        .lock()
        .map_or((WINDOW_CONTROL_NATIVE_X, WINDOW_CONTROL_NATIVE_Y), |val| *val)
}

pub fn set_window_control_tuning(tuning: WindowControlTuning) {
    if let Ok(mut active) = ACTIVE_WINDOW_CONTROL_TUNING.lock() {
        *active = tuning.clamped();
    }
}

pub fn active_window_control_tuning() -> WindowControlTuning {
    ACTIVE_WINDOW_CONTROL_TUNING
        .lock()
        .map_or_else(|poisoned| *poisoned.into_inner(), |active| *active)
}

pub fn set_window_control_group_hover(group_index: usize, hovered: bool) {
    if group_index >= u8::BITS as usize {
        return;
    }
    let bit = 1_u8 << group_index;
    if hovered {
        ACTIVE_WINDOW_CONTROL_HOVER.fetch_or(bit, Ordering::Relaxed);
    } else {
        ACTIVE_WINDOW_CONTROL_HOVER.fetch_and(!bit, Ordering::Relaxed);
    }
}

#[allow(dead_code)]
pub fn set_window_control_group_progress(group_index: usize, progress: f32) {
    if let Ok(mut values) = ACTIVE_WINDOW_CONTROL_PROGRESS.lock()
        && let Some(value) = values.get_mut(group_index)
    {
        *value = progress.clamp(0.0, 1.0);
    }
}

#[allow(dead_code)]
pub fn set_window_control_press_progress(id: GlassId, progress: f32) {
    let Some(index) = window_control_slot_index(id) else {
        return;
    };
    if let Ok(mut values) = ACTIVE_WINDOW_CONTROL_PRESS_PROGRESS.lock()
        && let Some(value) = values.get_mut(index)
    {
        *value = progress.clamp(0.0, 1.0);
    }
}

#[allow(dead_code)]
pub fn set_window_control_scale(id: GlassId, scale: f32) {
    let Some(index) = window_control_slot_index(id) else {
        return;
    };
    if let Ok(mut values) = ACTIVE_WINDOW_CONTROL_SCALE.lock()
        && let Some(value) = values.get_mut(index)
    {
        *value = scale.clamp(0.85, 1.30);
    }
}
pub fn window_control_group_hover_target(id: GlassId) -> f32 {
    let Some(group_index) = window_control_group_index(id) else {
        return 0.0;
    };
    let bit = 1_u8 << group_index;
    if ACTIVE_WINDOW_CONTROL_HOVER.load(Ordering::Relaxed) & bit != 0 { 1.0 } else { 0.0 }
}

pub fn window_control_group_progress(id: GlassId) -> Option<f32> {
    let group_index = window_control_group_index(id)?;
    ACTIVE_WINDOW_CONTROL_PROGRESS.lock().ok().and_then(|values| values.get(group_index).copied())
}

pub fn window_control_press_progress(id: GlassId) -> Option<f32> {
    let index = window_control_slot_index(id)?;
    ACTIVE_WINDOW_CONTROL_PRESS_PROGRESS.lock().ok().and_then(|values| values.get(index).copied())
}

pub fn window_control_scale(id: GlassId) -> f32 {
    let Some(index) = window_control_slot_index(id) else {
        return 1.0;
    };
    ACTIVE_WINDOW_CONTROL_SCALE
        .lock()
        .ok()
        .and_then(|values| values.get(index).copied())
        .unwrap_or(1.0)
}

pub fn window_control_slot_index(id: GlassId) -> Option<usize> {
    let group = window_control_group_index(id)?;
    let within_group = if let Some(index) =
        WINDOW_CONTROL_NATIVE_IDS.iter().position(|item| *item == id)
    {
        index
    } else if let Some(index) = WINDOW_CONTROL_REFERENCE_IDS.iter().position(|item| *item == id) {
        index
    } else if let Some(index) = WINDOW_CONTROL_LARGE_IDS.iter().position(|item| *item == id) {
        index
    } else if let Some(index) = WINDOW_CONTROL_INACTIVE_IDS.iter().position(|item| *item == id) {
        index
    } else {
        WINDOW_CONTROL_DISABLED_IDS.iter().position(|item| *item == id)?
    };
    Some(group * 3 + within_group)
}

pub fn window_control_group_index(id: GlassId) -> Option<usize> {
    if WINDOW_CONTROL_NATIVE_IDS.contains(&id) {
        Some(0)
    } else if WINDOW_CONTROL_REFERENCE_IDS.contains(&id) {
        Some(1)
    } else if WINDOW_CONTROL_LARGE_IDS.contains(&id) {
        Some(2)
    } else if WINDOW_CONTROL_INACTIVE_IDS.contains(&id) {
        Some(3)
    } else if WINDOW_CONTROL_DISABLED_IDS.contains(&id) {
        Some(4)
    } else {
        None
    }
}
pub fn traffic_lights_scene_for_viewport(
    scale_factor: f32,
    is_dark: bool,
    inactive: bool,
    interactions: &[(GlassId, GlassInteraction)],
) -> GlassScene {
    let mut scene = GlassScene::default();
    let scale_factor = scale_factor.max(1.0);
    let (origin_x, origin_y) = active_window_control_origin();
    let effective_tuning = if is_dark
        && active_window_control_tuning() == WindowControlTuning::for_scheme(false)
    {
        WindowControlTuning::for_scheme(true)
    } else {
        active_window_control_tuning()
    };
    push_traffic_light_group(
        &mut scene,
        &WINDOW_CONTROL_NATIVE_IDS,
        origin_x,
        origin_y,
        WINDOW_CONTROL_NATIVE_SIZE,
        WINDOW_CONTROL_GAP,
        inactive,
        false,
        is_dark,
        effective_tuning,
        interactions,
    );
    scale_scene(&mut scene, scale_factor);
    scene
}

pub fn push_traffic_light_group(
    scene: &mut GlassScene,
    ids: &[GlassId; 3],
    x: f32,
    y: f32,
    size: f32,
    gap: f32,
    inactive: bool,
    close_disabled: bool,
    is_dark: bool,
    tuning: WindowControlTuning,
    interactions: &[(GlassId, GlassInteraction)],
) {
    for (index, id) in ids.iter().copied().enumerate() {
        let base_x = x + index as f32 * (size + gap);
        let scale = window_control_scale(id);
        let visual_size = size * scale;
        let inset = (size - visual_size) * 0.5;
        let bounds = Rect::new(base_x + inset, y + inset, visual_size, visual_size);
        let interaction = interactions
            .iter()
            .find(|(interaction_id, _)| *interaction_id == id)
            .map_or_else(GlassInteraction::inactive, |(_, interaction)| *interaction);
        let focus = if inactive {
            window_control_group_progress(id).unwrap_or_else(|| interaction.hover.clamp(0.0, 1.0))
        } else {
            1.0
        };
        let hover = window_control_group_progress(id)
            .unwrap_or_else(|| interaction.hover.clamp(0.0, 1.0));
        let press = window_control_press_progress(id)
            .unwrap_or(0.0)
            .max(((scale - 1.0) / 0.18).clamp(0.0, 1.0));
        let base_color = traffic_light_color_for_scheme(
            index, inactive, close_disabled, is_dark, hover, press,
        );
        let active_color = traffic_light_color_for_scheme(
            index, false, close_disabled, is_dark, hover, press,
        );
        let display_color =
            if inactive { blend_color(base_color, active_color, focus) } else { base_color };
        let mut node = GlassNode::new(id, bounds)
            .shape(GlassShape::Circle)
            .material(traffic_light_material(
                display_color,
                visual_size,
                inactive,
                close_disabled,
                is_dark,
                tuning,
                focus,
            ))
            .interaction(interaction);
        node.z_index = 40;
        scene.push(node);
    }
}

pub fn blend_color(from: Color, to: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color::rgba(
        from.r + (to.r - from.r) * amount,
        from.g + (to.g - from.g) * amount,
        from.b + (to.b - from.b) * amount,
        from.a + (to.a - from.a) * amount,
    )
}
fn traffic_light_color_for_scheme(
    index: usize,
    inactive: bool,
    _close_disabled: bool,
    is_dark: bool,
    _hover: f32,
    press: f32,
) -> Color {
    if inactive {
        // AppKit removes the chromatic traffic-light pigments when the window
        // loses focus. Under dark mode, the graphite substrate is significantly
        // darker (0.32, 0.32, 0.35) than the light mode cool gray (0.78, 0.79, 0.82).
        return if is_dark {
            Color::rgba(0.32, 0.32, 0.35, 0.96)
        } else {
            Color::rgba(0.78, 0.79, 0.82, 0.96)
        };
    }
    // Calibrated source colours. These are deliberately saturated because the
    // glass body contributes a neutral substrate and transmission; a palette
    // matched only to the displayed centre samples would look washed out.
    // Press deepens the chromatic saturation; hover only transitions
    // inactive→active (handled by the caller), not individual highlight.
    let (base, press_rgb) = match index {
        0 => ((0.98, 0.34, 0.30), (1.00, 0.56, 0.50)),
        1 => ((0.98, 0.72, 0.14), (1.00, 0.84, 0.38)),
        _ => ((0.16, 0.77, 0.22), (0.34, 0.92, 0.42)),
    };
    let mut color = Color::rgba(base.0, base.1, base.2, 0.96);
    if press > 0.0 {
        color = blend_color(
            color,
            Color::rgba(press_rgb.0, press_rgb.1, press_rgb.2, 0.96),
            press,
        );
    }
    color
}

fn traffic_light_material(
    color: Color,
    _size: f32,
    inactive: bool,
    _close_disabled: bool,
    _is_dark: bool,
    tuning: WindowControlTuning,
    focus: f32,
) -> GlassMaterial {
    let focus = focus.clamp(0.0, 1.0);
    let muted_alpha = if inactive { 0.84 + 0.16 * focus } else { 1.0 };
    let muted_opacity = if inactive { 0.94 + 0.06 * focus } else { 1.0 };
    // Experimental baseline: the control is an exact circular SDF using the
    // stock physical glass material. Do not add a traffic-light-specific
    // border, light, glare, shadow, or size compensation here; the sphere's
    // refraction and Fresnel response must establish the edge by themselves.
    let mut material = GlassMaterial::regular();
    material.variant = GlassVariant::TrafficLightPhysical;
    material.blur.radius = tuning.blur_radius;
    material.traffic_light = TrafficLightStyle {
        substrate_coverage: tuning.substrate_coverage,
        lower_substrate_coverage: tuning.lower_substrate_coverage,
        lower_tint_coverage: tuning.lower_tint_coverage,
        angular_light: tuning.angular_light,
        light_angle: tuning.light_angle,
        light_softness: tuning.light_softness,
        body_thickness: tuning.body_thickness,
        internal_scattering: tuning.internal_scattering,
        side_edge_darkness: tuning.side_edge_darkness,
        side_edge_width: tuning.side_edge_width,
        edge_side_bias: tuning.edge_side_bias,
        edge_side_angle: tuning.edge_side_angle,
    };
    material.tint = Color::rgba(color.r, color.g, color.b, muted_alpha);
    material.refraction.strength = tuning.refraction_strength;
    material.fresnel.strength = tuning.fresnel_strength;
    material.whiteness = 0.0;
    material.opacity = tuning.opacity * muted_opacity;
    material
}
pub fn scale_scene(scene: &mut GlassScene, scale_factor: f32) {
    for node in scene.nodes_mut() {
        node.bounds.x *= scale_factor;
        node.bounds.y *= scale_factor;
        node.bounds.width *= scale_factor;
        node.bounds.height *= scale_factor;
        for fused_shape in &mut node.fused_shapes {
            fused_shape.bounds.x *= scale_factor;
            fused_shape.bounds.y *= scale_factor;
            fused_shape.bounds.width *= scale_factor;
            fused_shape.bounds.height *= scale_factor;
        }
        node.backdrop.bounds = node.visual_bounds();
        node.backdrop.padding *= scale_factor;
        node.backdrop.blur_radius *= scale_factor;
        node.material.blur.radius *= scale_factor;
        node.material.shadow.expand *= scale_factor;
        node.material.shadow.offset[0] *= scale_factor;
        node.material.shadow.offset[1] *= scale_factor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_traffic_lights_share_a_neutral_substrate() {
        let red = traffic_light_color_for_scheme(0, true, false, true, 0.0, 0.0);
        let yellow = traffic_light_color_for_scheme(1, true, false, true, 0.0, 0.0);
        let green = traffic_light_color_for_scheme(2, true, false, true, 0.0, 0.0);

        assert_eq!(red.r, yellow.r);
        assert_eq!(yellow.r, green.r);
        assert_eq!(red.g, yellow.g);
        assert_eq!(yellow.g, green.g);
        assert_eq!(red.b, yellow.b);
        assert_eq!(yellow.b, green.b);
        assert!(red.b > red.r);
    }

    #[test]
    fn traffic_lights_use_the_configured_physical_material() {
        let tuning = WindowControlTuning::default();
        let traffic = traffic_light_material(
            Color::rgba(1.0, 0.37, 0.34, 0.96),
            WINDOW_CONTROL_NATIVE_SIZE,
            false,
            false,
            false,
            tuning,
            1.0,
        );
        let stock = GlassMaterial::regular();

        assert_eq!(traffic.variant, GlassVariant::TrafficLightPhysical);
        assert_eq!(traffic.refraction.thickness, stock.refraction.thickness);
        assert_eq!(traffic.refraction.index, stock.refraction.index);
        assert_eq!(traffic.refraction.strength, tuning.refraction_strength);
        assert_eq!(traffic.fresnel.range, stock.fresnel.range);
        assert_eq!(traffic.fresnel.hardness, stock.fresnel.hardness);
        assert_eq!(traffic.fresnel.strength, tuning.fresnel_strength);
        assert_eq!(traffic.glare, stock.glare);
        assert_eq!(traffic.shadow, stock.shadow);
        assert_eq!(traffic.adaptive, stock.adaptive);
    }

    #[test]
    fn dark_window_controls_use_the_dark_material_preset() {
        let tuning = WindowControlTuning::for_scheme(true);

        assert_eq!(tuning.blur_radius, 17.0);
        assert_eq!(tuning.internal_scattering, 1.0);
        assert_eq!(tuning.side_edge_darkness, 0.0);
        assert_eq!(tuning.side_edge_width, 0.5);
        assert_eq!(tuning.opacity, 1.0);
        assert_eq!(tuning.refraction_strength, 0.0);
        assert_eq!(tuning.fresnel_strength, 0.39);
    }
}
