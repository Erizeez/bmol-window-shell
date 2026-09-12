//! Physical-glass material knobs for the traffic-light spheres.
//!
//! The traffic-light *effect* — the `TrafficLightPhysical` variant, its
//! [`liquid_glass_scene::TrafficLightStyle`], and the WGSL that implements it —
//! lives in liquid-rs. This module only holds the numbers that select and
//! shape that effect.

/// Runtime optical controls for the traffic-light physical material.
///
/// The values are copied into the semantic traffic-light material for one
/// frame, so changing a slider does not alter any other glass role.
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
    /// Whole-control brightening while the pointer is over the control.
    pub hover_gain: f32,
    /// Whole-control brightening while the control is held.
    pub press_gain: f32,
    /// Tint-hued brightness lift added while the control is held.
    pub press_lift: f32,
    /// Uniform incident field of the spherical centre light.
    pub uniform_light: f32,
    /// Extra centre-light release toward the thinner lower hemisphere.
    pub thin_light_gain: f32,
    /// Peak saturation lift at the optical centre of the droplet profile.
    pub core_lift: f32,
    /// Peak strength of the vertical axial glow.
    pub axial_glow: f32,
    /// Radial exponent of the core profile (`4.0` = reference `(1 - t)^4`).
    pub core_power: f32,
    /// Exponent of the vertical gradient; larger pools the light lower.
    pub vertical_power: f32,
    /// Exponent of the horizontal roll-off; smaller widens the lit centre.
    pub horizontal_power: f32,
}

impl WindowControlTuning {
    /// The calibrated light-mode default.
    #[must_use]
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
            hover_gain: 0.22,
            press_gain: 0.12,
            press_lift: 0.06,
            uniform_light: 0.045,
            thin_light_gain: 0.055,
            core_lift: 0.25,
            axial_glow: 0.46,
            core_power: 4.0,
            vertical_power: 1.35,
            horizontal_power: 0.25,
        }
    }

    /// Returns the scheme-specific material preset.
    ///
    /// The dark preset intentionally removes the side absorption and refraction
    /// response while increasing Fresnel, matching the measured dark-mode
    /// control treatment.
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
                hover_gain: 0.22,
                press_gain: 0.12,
                press_lift: 0.06,
                uniform_light: 0.045,
                thin_light_gain: 0.055,
                core_lift: 0.25,
                axial_glow: 0.46,
                core_power: 4.0,
                vertical_power: 1.35,
                horizontal_power: 0.25,
            }
        } else {
            Self::new()
        }
    }

    /// Resolves the material recipe for one sphere.
    ///
    /// `tint` is the calibrated display colour; the inactive window treatment
    /// (reduced alpha and opacity, both restored towards full as the group is
    /// revealed) is applied here so every consumer shares one formula.
    #[must_use]
    pub fn material_fields(
        self,
        tint: [f32; 4],
        inactive: bool,
        focus: f32,
    ) -> TrafficLightMaterialFields {
        let focus = focus.clamp(0.0, 1.0);
        let muted_alpha = if inactive { 0.84 + 0.16 * focus } else { 1.0 };
        let muted_opacity = if inactive { 0.94 + 0.06 * focus } else { 1.0 };
        TrafficLightMaterialFields {
            tint: [tint[0], tint[1], tint[2], tint[3] * muted_alpha],
            opacity: self.opacity * muted_opacity,
            blur_radius: self.blur_radius,
            refraction_strength: self.refraction_strength,
            fresnel_strength: self.fresnel_strength,
            interaction: [self.hover_gain, self.press_gain, self.press_lift],
            core_light: [
                self.uniform_light,
                self.thin_light_gain,
                self.core_lift,
                self.axial_glow,
                self.core_power,
                self.vertical_power,
                self.horizontal_power,
            ],
            style: TrafficLightStyleFields {
                substrate_coverage: self.substrate_coverage,
                lower_substrate_coverage: self.lower_substrate_coverage,
                lower_tint_coverage: self.lower_tint_coverage,
                angular_light: self.angular_light,
                light_angle: self.light_angle,
                light_softness: self.light_softness,
                body_thickness: self.body_thickness,
                internal_scattering: self.internal_scattering,
                side_edge_darkness: self.side_edge_darkness,
                side_edge_width: self.side_edge_width,
                edge_side_bias: self.edge_side_bias,
                edge_side_angle: self.edge_side_angle,
            },
        }
    }

    /// Clamps every knob into the range the shader expects.
    #[must_use]
    pub fn clamped(self) -> Self {
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
            hover_gain: self.hover_gain.clamp(0.0, 1.0),
            press_gain: self.press_gain.clamp(0.0, 1.0),
            press_lift: self.press_lift.clamp(0.0, 1.0),
            uniform_light: self.uniform_light.clamp(0.0, 1.0),
            thin_light_gain: self.thin_light_gain.clamp(0.0, 1.0),
            core_lift: self.core_lift.clamp(0.0, 1.0),
            axial_glow: self.axial_glow.clamp(0.0, 1.0),
            core_power: self.core_power.clamp(1.0, 8.0),
            vertical_power: self.vertical_power.clamp(0.25, 4.0),
            horizontal_power: self.horizontal_power.clamp(0.05, 2.0),
        }
    }
}

/// The physical-style knobs of one traffic-light sphere.
///
/// Deliberately plain numbers: a compositor on any `liquid-glass-scene`
/// release (or none at all) can consume the recipe without importing the
/// crate's scene types.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrafficLightStyleFields {
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
}

/// The complete material recipe for one traffic-light sphere.
///
/// [`WindowControlTuning::material_fields`] is the single place that decides
/// how the tuning knobs map onto a material, including the muted inactive
/// treatment. Consumers only copy these numbers into their own material type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrafficLightMaterialFields {
    /// Tint RGBA, already carrying the muted inactive alpha.
    pub tint: [f32; 4],
    pub opacity: f32,
    pub blur_radius: f32,
    pub refraction_strength: f32,
    pub fresnel_strength: f32,
    /// Pointer-engagement gains, in the order hover / press / press lift.
    pub interaction: [f32; 3],
    /// Centre light, in the order uniform, thin, core lift, axial glow, core
    /// power, vertical power, horizontal power.
    pub core_light: [f32; 7],
    pub style: TrafficLightStyleFields,
}

impl Default for WindowControlTuning {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_preset_is_the_measured_dark_treatment() {
        let tuning = WindowControlTuning::for_scheme(true);

        assert_eq!(tuning.blur_radius, 17.0);
        assert_eq!(tuning.internal_scattering, 1.0);
        assert_eq!(tuning.side_edge_darkness, 0.0);
        assert_eq!(tuning.side_edge_width, 0.5);
        assert_eq!(tuning.opacity, 1.0);
        assert_eq!(tuning.refraction_strength, 0.0);
        assert_eq!(tuning.fresnel_strength, 0.39);
    }

    #[test]
    fn material_recipe_applies_the_inactive_muting() {
        let tuning = WindowControlTuning::default();
        let active = tuning.material_fields([1.0, 0.5, 0.25, 0.96], false, 0.0);
        let dark_inactive = tuning.material_fields([1.0, 0.5, 0.25, 0.96], true, 0.0);
        let revealed = tuning.material_fields([1.0, 0.5, 0.25, 0.96], true, 1.0);

        assert_eq!(active.tint[3], 0.96);
        assert_eq!(active.opacity, tuning.opacity);
        assert!(dark_inactive.tint[3] < active.tint[3]);
        assert!(dark_inactive.opacity < active.opacity);
        // A revealed inactive window is restored towards the active treatment
        // without ever exceeding it.
        assert!(revealed.tint[3] > dark_inactive.tint[3]);
        assert!(revealed.tint[3] <= active.tint[3] + f32::EPSILON);
        assert!(revealed.opacity > dark_inactive.opacity);
        assert_eq!(revealed.interaction, [0.22, 0.12, 0.06]);
        assert_eq!(
            revealed.core_light,
            [0.045, 0.055, 0.25, 0.46, 4.0, 1.35, 0.25]
        );
        assert_eq!(revealed.style.substrate_coverage, tuning.substrate_coverage);
        assert_eq!(revealed.blur_radius, tuning.blur_radius);
        assert_eq!(revealed.refraction_strength, tuning.refraction_strength);
        assert_eq!(revealed.fresnel_strength, tuning.fresnel_strength);
    }

    #[test]
    fn clamping_keeps_every_knob_in_range() {
        let wild = WindowControlTuning {
            blur_radius: -5.0,
            opacity: 9.0,
            substrate_coverage: 4.0,
            lower_substrate_coverage: -1.0,
            lower_tint_coverage: 2.0,
            angular_light: 99.0,
            light_angle: -2.0,
            light_softness: 3.0,
            body_thickness: 0.0,
            internal_scattering: 5.0,
            side_edge_darkness: 9.0,
            side_edge_width: 0.0,
            edge_side_bias: 4.0,
            edge_side_angle: 0.0,
            refraction_strength: 3.0,
            fresnel_strength: -1.0,
            hover_gain: 9.0,
            press_gain: -1.0,
            press_lift: 4.0,
            uniform_light: 5.0,
            thin_light_gain: -2.0,
            core_lift: 3.0,
            axial_glow: -1.0,
            core_power: 99.0,
            vertical_power: 0.0,
            horizontal_power: 9.0,
        }
        .clamped();

        assert_eq!(wild, wild.clamped());
        assert_eq!(wild.blur_radius, 0.0);
        assert_eq!(wild.opacity, 1.0);
        assert_eq!(wild.side_edge_width, 0.25);
        assert_eq!(wild.edge_side_angle, 10.0);
        assert_eq!(wild.fresnel_strength, 0.0);
        assert_eq!(wild.hover_gain, 1.0);
        assert_eq!(wild.press_gain, 0.0);
        assert_eq!(wild.press_lift, 1.0);
        assert_eq!(wild.uniform_light, 1.0);
        assert_eq!(wild.thin_light_gain, 0.0);
        assert_eq!(wild.core_lift, 1.0);
        assert_eq!(wild.axial_glow, 0.0);
        assert_eq!(wild.core_power, 8.0);
        assert_eq!(wild.vertical_power, 0.25);
        assert_eq!(wild.horizontal_power, 2.0);
    }
}
