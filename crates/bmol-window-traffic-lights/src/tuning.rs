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
    pub refraction_strength: f32,
    pub fresnel_strength: f32,
    /// Whole-control brightening while the pointer is over the control.
    pub hover_gain: f32,
    /// Whole-control brightening while the control is held.
    pub press_gain: f32,
    /// Tint-hued brightness lift added while the control is held.
    pub press_lift: f32,
    /// Droplet profile control points of the flat bead (all zero is `(1-t)^4`).
    pub bead_b1: f32,
    pub bead_b2: f32,
    pub bead_b3: f32,
    /// Vertical axial glow strength of the flat bead.
    pub bead_center_glow: f32,
    /// Core saturation lift of the flat bead.
    pub bead_saturation_lift: f32,
    /// Bright-edge strength (dark appearance only).
    pub bead_highlight_intensity: f32,
    /// Dark-rim strength (light appearance only).
    pub bead_dark_rim_intensity: f32,
    /// Bright-edge span factor.
    pub bead_core_span_factor: f32,
    /// Dark-rim span factor.
    pub bead_rim_span_factor: f32,
    /// Caustic multiplier of the axial glow, light appearance.
    pub bead_caustic_light: f32,
    /// Caustic multiplier of the axial glow, dark appearance.
    pub bead_caustic_dark: f32,
}

impl WindowControlTuning {
    /// The calibrated light-mode default.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            blur_radius: 17.0,
            opacity: 1.0,
            refraction_strength: 0.5,
            fresnel_strength: 0.0,
            hover_gain: 0.22,
            press_gain: 0.12,
            press_lift: 0.06,
            bead_b1: 0.0,
            bead_b2: 0.0,
            bead_b3: 0.0,
            bead_center_glow: 1.0,
            bead_saturation_lift: 0.25,
            bead_highlight_intensity: 0.85,
            bead_dark_rim_intensity: 2.0,
            bead_core_span_factor: 1.0,
            bead_rim_span_factor: 1.0,
            bead_caustic_light: 1.10,
            bead_caustic_dark: 0.80,
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
                opacity: 1.0,
                refraction_strength: 0.0,
                fresnel_strength: 0.39,
                hover_gain: 0.22,
                press_gain: 0.12,
                press_lift: 0.06,
                bead_b1: 0.0,
                bead_b2: 0.0,
                bead_b3: 0.0,
                bead_center_glow: 1.0,
                bead_saturation_lift: 0.25,
                bead_highlight_intensity: 0.85,
                bead_dark_rim_intensity: 2.0,
                bead_core_span_factor: 1.0,
                bead_rim_span_factor: 1.0,
                bead_caustic_light: 1.10,
                bead_caustic_dark: 0.80,
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
            bead: [
                self.bead_b1,
                self.bead_b2,
                self.bead_b3,
                self.bead_center_glow,
                self.bead_saturation_lift,
                self.bead_highlight_intensity,
                self.bead_dark_rim_intensity,
                self.bead_core_span_factor,
                self.bead_rim_span_factor,
                self.bead_caustic_light,
                self.bead_caustic_dark,
            ],
        }
    }

    /// Clamps every knob into the range the shader expects.
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            blur_radius: self.blur_radius.clamp(0.0, 80.0),
            opacity: self.opacity.clamp(0.0, 1.0),
            refraction_strength: self.refraction_strength.clamp(0.0, 1.0),
            fresnel_strength: self.fresnel_strength.clamp(0.0, 1.0),
            hover_gain: self.hover_gain.clamp(0.0, 1.0),
            press_gain: self.press_gain.clamp(0.0, 1.0),
            press_lift: self.press_lift.clamp(0.0, 1.0),
            bead_b1: self.bead_b1.clamp(0.0, 2.0),
            bead_b2: self.bead_b2.clamp(0.0, 2.0),
            bead_b3: self.bead_b3.clamp(0.0, 1.0),
            bead_center_glow: self.bead_center_glow.clamp(0.0, 2.0),
            bead_saturation_lift: self.bead_saturation_lift.clamp(0.0, 0.5),
            bead_highlight_intensity: self.bead_highlight_intensity.clamp(0.0, 2.0),
            bead_dark_rim_intensity: self.bead_dark_rim_intensity.clamp(0.0, 3.0),
            bead_core_span_factor: self.bead_core_span_factor.clamp(0.5, 2.0),
            bead_rim_span_factor: self.bead_rim_span_factor.clamp(0.5, 2.0),
            bead_caustic_light: self.bead_caustic_light.clamp(0.0, 2.0),
            bead_caustic_dark: self.bead_caustic_dark.clamp(0.0, 2.0),
        }
    }
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
    /// Reference bead, in the order b1, b2, b3, centre glow, saturation lift,
    /// highlight, dark rim, core span, rim span, caustic light, caustic dark.
    pub bead: [f32; 11],
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
        assert_eq!(revealed.blur_radius, tuning.blur_radius);
        assert_eq!(revealed.refraction_strength, tuning.refraction_strength);
        assert_eq!(revealed.fresnel_strength, tuning.fresnel_strength);
    }

    #[test]
    fn clamping_keeps_every_knob_in_range() {
        let wild = WindowControlTuning {
            blur_radius: -5.0,
            opacity: 9.0,
            refraction_strength: 3.0,
            fresnel_strength: -1.0,
            hover_gain: 9.0,
            press_gain: -1.0,
            press_lift: 4.0,
            bead_b1: 5.0,
            bead_b2: -1.0,
            bead_b3: 5.0,
            bead_center_glow: 9.0,
            bead_saturation_lift: 9.0,
            bead_highlight_intensity: -1.0,
            bead_dark_rim_intensity: 9.0,
            bead_core_span_factor: 0.0,
            bead_rim_span_factor: 9.0,
            bead_caustic_light: -1.0,
            bead_caustic_dark: 9.0,
        }
        .clamped();

        assert_eq!(wild, wild.clamped());
        assert_eq!(wild.blur_radius, 0.0);
        assert_eq!(wild.opacity, 1.0);
        assert_eq!(wild.fresnel_strength, 0.0);
        assert_eq!(wild.hover_gain, 1.0);
        assert_eq!(wild.press_gain, 0.0);
        assert_eq!(wild.press_lift, 1.0);
        assert_eq!(wild.bead_b1, 2.0);
        assert_eq!(wild.bead_b2, 0.0);
        assert_eq!(wild.bead_b3, 1.0);
        assert_eq!(wild.bead_center_glow, 2.0);
        assert_eq!(wild.bead_saturation_lift, 0.5);
        assert_eq!(wild.bead_highlight_intensity, 0.0);
        assert_eq!(wild.bead_dark_rim_intensity, 3.0);
        assert_eq!(wild.bead_core_span_factor, 0.5);
        assert_eq!(wild.bead_rim_span_factor, 2.0);
        assert_eq!(wild.bead_caustic_light, 0.0);
        assert_eq!(wild.bead_caustic_dark, 2.0);
    }
}
