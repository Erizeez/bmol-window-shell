//! Renderer-agnostic Liquid Glass scene construction for the traffic lights.
//!
//! The scene builder is the only place that turns interaction values into
//! [`GlassNode`]s. It reads one [`TrafficLightsFrame`] snapshot per call, so the
//! spheres and the Iced glyph layer are always driven by the same numbers.

use liquid_glass_scene::{
    BeadStyle, Color, CoreLight, GlassId, GlassInteraction, GlassMaterial, GlassNode, GlassScene, GlassShape,
    GlassVariant, InteractionResponse, Rect, RimProfile, TrafficLightStyle,
};

use crate::interaction::{
    active_window_control_origin, snapshot, window_control_group_progress,
};
use crate::layout::{
    WINDOW_CONTROL_GAP, WINDOW_CONTROL_NATIVE_IDS, WINDOW_CONTROL_NATIVE_SIZE,
};
use crate::palette::{blend_color, traffic_light_source_colors};
use crate::tuning::WindowControlTuning;

/// Builds the calibrated source colour for one control.
///
/// Inactive windows lose the chromatic pigment entirely, as AppKit does. Under
/// dark mode the graphite substrate is significantly darker than the light mode
/// cool gray.
#[must_use]
pub fn traffic_light_source_color(
    index: usize,
    inactive: bool,
    is_dark: bool,
    press: f32,
) -> Color {
    if inactive {
        return if is_dark {
            Color::rgba(0.32, 0.32, 0.35, 0.96)
        } else {
            Color::rgba(0.78, 0.79, 0.82, 0.96)
        };
    }
    let (base, pressed) = traffic_light_source_colors(index, press);
    if press > 0.0 { blend_color(base, pressed, press) } else { base }
}

/// Builds the physical-glass material for one traffic-light sphere.
///
/// Experimental baseline: the control is an exact circular SDF using the stock
/// physical glass material. Do not add a traffic-light-specific border, light,
/// glare, shadow, or size compensation here; the sphere's refraction and
/// Fresnel response must establish the edge by themselves.
#[must_use]
pub fn traffic_light_material(
    color: Color,
    inactive: bool,
    is_dark: bool,
    tuning: WindowControlTuning,
    focus: f32,
) -> GlassMaterial {
    let fields = tuning.material_fields([color.r, color.g, color.b, color.a], inactive, focus);

    let mut material = GlassMaterial::regular();
    // Reference bead or physical glass. The bead variant is not finished yet:
    // its screen-space geometry is unverified and it returns before the normal
    // composition, so switching it on renders a wrong (and mis-placed) control.
    // Flip this to `true` to compare the two.
    const USE_REFERENCE_BEAD: bool = false;
    material.variant = if USE_REFERENCE_BEAD {
        GlassVariant::TrafficLightBead
    } else {
        GlassVariant::TrafficLightPhysical
    };
    material.bead = BeadStyle {
        b1: fields.bead[0],
        b2: fields.bead[1],
        b3: fields.bead[2],
        center_glow: fields.bead[3],
        saturation_lift: fields.bead[4],
        highlight_intensity: fields.bead[5],
        dark_rim_intensity: fields.bead[6],
        core_span_factor: fields.bead[7],
        rim_span_factor: fields.bead[8],
        caustic_light: fields.bead[9],
        caustic_dark: fields.bead[10],
        mode_dark: if is_dark { 1.0 } else { 0.0 },
    };
    material.blur.radius = fields.blur_radius;
    material.traffic_light = TrafficLightStyle {
        substrate_coverage: fields.style.substrate_coverage,
        lower_substrate_coverage: fields.style.lower_substrate_coverage,
        lower_tint_coverage: fields.style.lower_tint_coverage,
        angular_light: fields.style.angular_light,
        light_angle: fields.style.light_angle,
        light_softness: fields.style.light_softness,
        body_thickness: fields.style.body_thickness,
        internal_scattering: fields.style.internal_scattering,
        side_edge_darkness: fields.style.side_edge_darkness,
        side_edge_width: fields.style.side_edge_width,
        edge_side_bias: fields.style.edge_side_bias,
        edge_side_angle: fields.style.edge_side_angle,
    };
    material.tint = Color::rgba(fields.tint[0], fields.tint[1], fields.tint[2], fields.tint[3]);
    // Pointer engagement and centre light travel through the material, so a
    // playground can tune them without the shader knowing about tuning structs.
    material.interaction = InteractionResponse {
        hover_gain: fields.interaction[0],
        press_gain: fields.interaction[1],
        press_lift: fields.interaction[2],
    };
    material.core_light = CoreLight {
        uniform_light: fields.core_light[0],
        thin_light_gain: fields.core_light[1],
        core_lift: fields.core_light[2],
        axial_glow: fields.core_light[3],
        core_power: fields.core_light[4],
        vertical_power: fields.core_light[5],
        horizontal_power: fields.core_light[6],
    };
    material.rim_profile = RimProfile {
        lateral_power: fields.rim_profile[0],
        vertical_floor: fields.rim_profile[1],
        grazing_power: fields.rim_profile[2],
    };
    material.refraction.strength = fields.refraction_strength;
    material.fresnel.strength = fields.fresnel_strength;
    material.whiteness = 0.0;
    material.opacity = fields.opacity;
    material
}

/// Centre-light share of the secondary controls (yellow and green).
///
/// Measured from the reference appearance: red carries the full centre glow
/// while yellow and green reach only this fraction of it. The reference bakes
/// this share into every state, so it is a property of the control rather than
/// of the pointer response.
pub const TRAFFIC_LIGHT_SECONDARY_GLOW_SHARE: f32 = 0.6;

/// Bounds of control `index` inside a group, given its current press scale.
///
/// The sphere grows about the circle's centre: the visual diameter scales and
/// the inset keeps the centres fixed, which is what makes the neighbouring
/// controls stay put while one is pressed. Pure arithmetic, no glass types, so
/// every compositor places the spheres identically.
#[must_use]
pub fn sphere_bounds(x: f32, y: f32, size: f32, gap: f32, index: usize, scale: f32) -> [f32; 4] {
    let visual_size = size * scale;
    let inset = (size - visual_size) * 0.5;
    let base_x = x + index as f32 * (size + gap);
    [base_x + inset, y + inset, visual_size, visual_size]
}

/// Pushes one traffic-light group into `scene` from the current frame snapshot.
///
/// `interactions` carries compositor-side extras (parallax, pointer position)
/// only; hover, press, and scale always come from the shared interaction store,
/// which the state machine owns.
#[allow(clippy::too_many_arguments)]
pub fn push_traffic_light_group(
    scene: &mut GlassScene,
    ids: &[GlassId; 3],
    x: f32,
    y: f32,
    size: f32,
    gap: f32,
    inactive: bool,
    // Reserved: the unavailable-control treatment is currently owned by the
    // glyph layer (the edited/disabled dot) rather than the glass material.
    _close_disabled: bool,
    is_dark: bool,
    tuning: WindowControlTuning,
    interactions: &[(GlassId, GlassInteraction)],
) {
    let frame = snapshot();
    for (index, id) in ids.iter().copied().enumerate() {
        let values = frame.interaction(id);
        let [bx, by, bw, bh] = sphere_bounds(x, y, size, gap, index, values.scale);
        let bounds = Rect::new(bx, by, bw, bh);
        // The press changes the sphere's drawn radius, not the area of backdrop
        // the material samples. The node's backdrop region starts as the shape
        // bounds, so letting it follow the scale made a blurred rectangle grow
        // and shrink behind the control with the spring; pin it to the resting
        // footprint instead.
        let [rx, ry, rw, rh] = sphere_bounds(x, y, size, gap, index, 1.0);
        let resting_bounds = Rect::new(rx, ry, rw, rh);

        let interaction = interactions
            .iter()
            .find(|(interaction_id, _)| *interaction_id == id)
            .map_or_else(GlassInteraction::inactive, |(_, interaction)| *interaction);

        let focus = if inactive { values.hover } else { 1.0 };
        let base_color = traffic_light_source_color(index, inactive, is_dark, values.press);
        let active_color = traffic_light_source_color(index, false, is_dark, values.press);
        let display_color =
            if inactive { blend_color(base_color, active_color, focus) } else { base_color };

        // The reference appearance does not give every control the same centre
        // light: red carries the full glow while yellow and green stop at 60%
        // of it. That share is a property of the resting control -- it applies
        // in every state, hover and press alike -- so it scales the centre
        // light rather than the pointer response.
        let glow_share = if index == 0 { 1.0 } else { TRAFFIC_LIGHT_SECONDARY_GLOW_SHARE };
        let mut material = traffic_light_material(display_color, inactive, is_dark, tuning, focus);
        material.bead.center_glow *= glow_share;

        let mut node = GlassNode::new(id, bounds)
            .shape(GlassShape::Circle)
            .material(material)
            .interaction(interaction);
        node.backdrop.bounds = resting_bounds;
        node.z_index = 40;
        scene.push(node);
    }
}

/// Builds the standalone native traffic-light group for a window viewport.
///
/// This is the single-group scene used by real windows: the three measured
/// controls at the origin published by the Iced glyph row.
#[must_use]
pub fn traffic_lights_scene_for_viewport(
    scale_factor: f32,
    is_dark: bool,
    inactive: bool,
    interactions: &[(GlassId, GlassInteraction)],
) -> GlassScene {
    let mut scene = GlassScene::default();
    let scale_factor = scale_factor.max(1.0);
    let frame = snapshot();
    let effective_tuning = if is_dark && frame.tuning == WindowControlTuning::for_scheme(false) {
        WindowControlTuning::for_scheme(true)
    } else {
        frame.tuning
    };
    push_traffic_light_group(
        &mut scene,
        &WINDOW_CONTROL_NATIVE_IDS,
        active_window_control_origin().0,
        active_window_control_origin().1,
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

/// Converts a logical-point scene into device pixels.
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

/// Group glyph reveal for the group owning `id`, clamped to `0..=1`.
#[must_use]
pub fn group_hover(id: GlassId) -> f32 {
    window_control_group_progress(id).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::{publish_group, reset_groups};
    use crate::layout::{WINDOW_CONTROL_GROUPS, WINDOW_CONTROL_NATIVE_Y};
    use crate::state::TrafficLightsState;

    #[test]
    fn inactive_lights_share_a_neutral_substrate() {
        let red = traffic_light_source_color(0, true, true, 0.0);
        let yellow = traffic_light_source_color(1, true, true, 0.0);
        let green = traffic_light_source_color(2, true, true, 0.0);

        assert_eq!(red, yellow);
        assert_eq!(yellow, green);
        assert!(red.b > red.r);
    }

    #[test]
    fn traffic_lights_use_the_physical_material_with_bead_parameters_ready() {
        let tuning = WindowControlTuning::default();
        let traffic =
            traffic_light_material(Color::rgba(1.0, 0.37, 0.34, 0.96), false, false, tuning, 1.0);
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
    fn sphere_bounds_grow_about_the_circle_centre() {
        let rest = sphere_bounds(10.0, 20.0, 14.0, 9.0, 1, 1.0);
        assert_eq!(rest, [33.0, 20.0, 14.0, 14.0]);

        let pressed = sphere_bounds(10.0, 20.0, 14.0, 9.0, 1, 1.18);
        let centre = |b: [f32; 4]| (b[0] + b[2] * 0.5, b[1] + b[3] * 0.5);
        assert!((centre(pressed).0 - centre(rest).0).abs() < 1e-4);
        assert!((centre(pressed).1 - centre(rest).1).abs() < 1e-4);
        assert!(pressed[2] > rest[2]);
        // Neighbours must not move when one control is pressed.
        assert_eq!(sphere_bounds(10.0, 20.0, 14.0, 9.0, 0, 1.0), [10.0, 20.0, 14.0, 14.0]);
        assert_eq!(sphere_bounds(10.0, 20.0, 14.0, 9.0, 2, 1.0), [56.0, 20.0, 14.0, 14.0]);
    }

    #[test]
    fn the_scene_follows_the_published_interaction() {
        reset_groups();
        let mut state = TrafficLightsState::new();
        state.on_group_hover(true);
        state.on_press_start(0);
        for _ in 0..60 {
            state.advance(1.0 / 60.0);
        }
        publish_group(2, &state);

        let mut scene = GlassScene::default();
        push_traffic_light_group(
            &mut scene,
            &WINDOW_CONTROL_GROUPS[2].1,
            240.0,
            292.0,
            64.0,
            12.0,
            false,
            false,
            false,
            WindowControlTuning::default(),
            &[],
        );

        let nodes = scene.nodes();
        assert_eq!(nodes.len(), 3);
        let pressed = nodes[0].bounds;
        assert!(pressed.width > 64.0, "the pressed sphere must be scaled up");
        assert!(pressed.width < 64.0 * 1.2);
        assert_eq!(nodes[1].bounds.width, 64.0);
        reset_groups();
    }

    #[test]
    fn only_red_carries_the_full_centre_glow() {
        reset_groups();
        let mut state = TrafficLightsState::new();
        state.on_group_hover(true);
        publish_group(0, &state);

        let mut scene = GlassScene::default();
        push_traffic_light_group(
            &mut scene,
            &WINDOW_CONTROL_GROUPS[0].1,
            10.0,
            18.0,
            14.0,
            9.0,
            false,
            false,
            false,
            WindowControlTuning::default(),
            &[],
        );

        let nodes = scene.nodes();
        let base = WindowControlTuning::default().bead_center_glow;
        let expected = base * TRAFFIC_LIGHT_SECONDARY_GLOW_SHARE;

        let red = nodes[0].material.bead.center_glow;
        let yellow = nodes[1].material.bead.center_glow;
        let green = nodes[2].material.bead.center_glow;
        assert!((red - base).abs() < 1e-6, "red keeps the full glow: {red} vs {base}");
        assert!((yellow - expected).abs() < 1e-6, "yellow: {yellow} vs {expected}");
        assert!((green - expected).abs() < 1e-6, "green: {green} vs {expected}");

        // The pointer response is identical across the group, in every state.
        for node in nodes {
            assert!((node.material.interaction.hover_gain - base.max(0.0) * 0.0
                - WindowControlTuning::default().hover_gain)
                .abs()
                < 1e-6);
        }
        reset_groups();
    }

    #[test]
    fn the_backdrop_region_does_not_follow_the_press_scale() {
        reset_groups();
        let mut state = TrafficLightsState::new();
        state.on_press_start(0);
        for _ in 0..30 {
            state.advance(1.0 / 60.0);
        }
        publish_group(0, &state);

        let mut scene = GlassScene::default();
        push_traffic_light_group(
            &mut scene,
            &WINDOW_CONTROL_GROUPS[0].1,
            10.0,
            18.0,
            14.0,
            9.0,
            false,
            false,
            false,
            WindowControlTuning::default(),
            &[],
        );

        let pressed = &scene.nodes()[0];
        assert!(pressed.bounds.width > 14.0, "the sphere itself must grow");
        assert!(
            (pressed.backdrop.bounds.width - 14.0).abs() < 1e-4,
            "the sampled backdrop must stay at the resting size, got {}",
            pressed.backdrop.bounds.width
        );
        reset_groups();
    }

    #[test]
    fn native_scene_uses_the_published_origin() {
        crate::interaction::set_window_control_origin(12.0, 20.0);
        let scene = traffic_lights_scene_for_viewport(2.0, false, false, &[]);
        let nodes = scene.nodes();
        assert_eq!(nodes.len(), 3);
        assert!((nodes[0].bounds.x - 24.0).abs() < 0.001);
        assert!((nodes[0].bounds.y - 40.0).abs() < 0.001);
        crate::interaction::set_window_control_origin(
            crate::layout::WINDOW_CONTROL_NATIVE_X,
            WINDOW_CONTROL_NATIVE_Y,
        );
    }
}
