//! The shared traffic-light interaction frame.
//!
//! The state machine owns the animation; this module is the *only* channel
//! through which the Iced layer publishes it to the compositor. The compositor
//! takes one [`snapshot`] per frame, so a scene can never mix values from two
//! different ticks, and there is exactly one writer per value.

use std::sync::Mutex;

use liquid_glass_scene::GlassId;

use crate::layout::{
    GROUP_COUNT, GROUP_LEN, WINDOW_CONTROL_GROUPS, WINDOW_CONTROL_NATIVE_X,
    WINDOW_CONTROL_NATIVE_Y, WINDOW_CONTROL_RAW_GROUPS,
};
use crate::state::{PRESS_SCALE_REST, TrafficLightsState};
use crate::tuning::WindowControlTuning;

/// Number of individual controls across every sample group.
pub const SLOT_COUNT: usize = GROUP_COUNT * GROUP_LEN;

/// Interaction values for one control, as the compositor consumes them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrafficLightInteraction {
    /// Group glyph reveal, `0..=1`.
    pub hover: f32,
    /// Press tint, `0..=1`.
    pub press: f32,
    /// Press scale, `1.0` at rest.
    pub scale: f32,
}

impl TrafficLightInteraction {
    /// The settled, untouched control.
    pub const REST: Self = Self { hover: 0.0, press: 0.0, scale: PRESS_SCALE_REST };

    /// Clamps every field into its valid range.
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            hover: self.hover.clamp(0.0, 1.0),
            press: self.press.clamp(0.0, 1.0),
            scale: self.scale.clamp(0.85, 1.30),
        }
    }
}

impl Default for TrafficLightInteraction {
    fn default() -> Self {
        Self::REST
    }
}

/// One consistent frame of traffic-light interaction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrafficLightsFrame {
    /// Where the measured sample sits inside the surface.
    pub origin: (f32, f32),
    /// Material knobs shared by every sample.
    pub tuning: WindowControlTuning,
    /// Per-group values, indexed like [`WINDOW_CONTROL_GROUPS`].
    pub groups: [[TrafficLightInteraction; GROUP_LEN]; GROUP_COUNT],
}

impl Default for TrafficLightsFrame {
    fn default() -> Self {
        Self {
            origin: (WINDOW_CONTROL_NATIVE_X, WINDOW_CONTROL_NATIVE_Y),
            tuning: WindowControlTuning::for_scheme(true),
            groups: [[TrafficLightInteraction::REST; GROUP_LEN]; GROUP_COUNT],
        }
    }
}

impl TrafficLightsFrame {
    /// Interaction values for one control, addressed by its glass id.
    #[must_use]
    pub fn interaction(&self, id: GlassId) -> TrafficLightInteraction {
        window_control_slot_index(id)
            .map_or(TrafficLightInteraction::REST, |slot| self.slot(slot))
    }

    /// Interaction values for a flat slot index (`group * 3 + within`).
    #[must_use]
    pub fn slot(&self, slot: usize) -> TrafficLightInteraction {
        let group = slot / GROUP_LEN;
        let within = slot % GROUP_LEN;
        self.groups
            .get(group)
            .and_then(|values| values.get(within))
            .copied()
            .unwrap_or(TrafficLightInteraction::REST)
    }

    /// Overwrites one group from a state machine instance.
    pub fn publish(&mut self, group_index: usize, state: &TrafficLightsState) {
        let Some(group) = self.groups.get_mut(group_index) else {
            return;
        };
        for (within, value) in group.iter_mut().enumerate() {
            *value = TrafficLightInteraction {
                hover: state.hover_progress,
                press: state.press_tint(within),
                scale: state.press_scale(within),
            };
        }
    }
}

static FRAME: Mutex<TrafficLightsFrame> = Mutex::new(TrafficLightsFrame {
    origin: (WINDOW_CONTROL_NATIVE_X, WINDOW_CONTROL_NATIVE_Y),
    tuning: WindowControlTuning::for_scheme(true),
    groups: [[TrafficLightInteraction::REST; GROUP_LEN]; GROUP_COUNT],
});

fn with_frame<T>(f: impl FnOnce(&mut TrafficLightsFrame) -> T, fallback: T) -> T {
    FRAME.lock().map_or(fallback, |mut frame| f(&mut frame))
}

/// Publishes one group's interaction values from its state machine.
pub fn publish_group(group_index: usize, state: &TrafficLightsState) {
    with_frame(|frame| frame.publish(group_index, state), ());
}

/// Publishes every group in one pass.
pub fn publish_groups(states: &[TrafficLightsState]) {
    with_frame(
        |frame| {
            for (group_index, state) in states.iter().enumerate().take(GROUP_COUNT) {
                frame.publish(group_index, state);
            }
        },
        (),
    );
}

/// Resets every group to rest.
pub fn reset_groups() {
    with_frame(
        |frame| {
            for group in &mut frame.groups {
                *group = [TrafficLightInteraction::REST; GROUP_LEN];
            }
        },
        (),
    );
}

/// Compatibility: writes one control's press scale directly.
///
/// A host that advances its own press spring can push the resulting value
/// instead of publishing a whole [`TrafficLightsState`]. Prefer
/// [`publish_group`]; this exists so a compositor that owns the spring keeps
/// working without a second copy of the state machine.
pub fn set_window_control_scale(id: GlassId, scale: f32) {
    with_frame(
        |frame| {
            if let Some(slot) = window_control_slot_index(id) {
                frame.groups[slot / GROUP_LEN][slot % GROUP_LEN].scale = scale.clamp(0.85, 1.30);
            }
        },
        (),
    );
}

/// Compatibility: writes one control's press tint directly.
pub fn set_window_control_press_progress(id: GlassId, press: f32) {
    with_frame(
        |frame| {
            if let Some(slot) = window_control_slot_index(id) {
                frame.groups[slot / GROUP_LEN][slot % GROUP_LEN].press = press.clamp(0.0, 1.0);
            }
        },
        (),
    );
}

/// Compatibility: writes one group's glyph reveal directly.
pub fn set_window_control_group_progress(group_index: usize, progress: f32) {
    with_frame(
        |frame| {
            if let Some(group) = frame.groups.get_mut(group_index) {
                let progress = progress.clamp(0.0, 1.0);
                for value in group.iter_mut() {
                    value.hover = progress;
                }
            }
        },
        (),
    );
}

/// Compatibility: sets one group's reveal from a hover flag.
pub fn set_window_control_group_hover(group_index: usize, hovered: bool) {
    set_window_control_group_progress(group_index, if hovered { 1.0 } else { 0.0 });
}

/// Compatibility: the reveal a group is heading towards.
///
/// The frame only carries the resolved reveal, so this reports it as the
/// target too. A host that animates its own reveal toward this value will
/// simply reach it faster than it would against a separate target.
#[must_use]
pub fn window_control_group_hover_target(id: GlassId) -> f32 {
    window_control_group_progress(id).unwrap_or(0.0)
}

/// Takes one consistent frame for the compositor.
#[must_use]
pub fn snapshot() -> TrafficLightsFrame {
    FRAME.lock().map_or_else(|poisoned| *poisoned.into_inner(), |frame| *frame)
}

/// Sets the measured origin of the native sample.
///
/// The Iced glyph row publishes its real layout origin here so the spheres and
/// the symbols share one geometry source instead of two estimates.
pub fn set_window_control_origin(x: f32, y: f32) {
    with_frame(|frame| frame.origin = (x, y), ());
}

/// Returns the measured origin of the native sample.
#[must_use]
pub fn active_window_control_origin() -> (f32, f32) {
    snapshot().origin
}

/// Replaces the shared material tuning.
pub fn set_window_control_tuning(tuning: WindowControlTuning) {
    let tuning = tuning.clamped();
    with_frame(|frame| frame.tuning = tuning, ());
}

/// Returns the shared material tuning.
#[must_use]
pub fn active_window_control_tuning() -> WindowControlTuning {
    snapshot().tuning
}

/// Group glyph reveal for the group owning `id`.
#[must_use]
pub fn window_control_group_progress(id: GlassId) -> Option<f32> {
    window_control_group_index(id).map(|group| snapshot().groups[group][0].hover)
}

/// Press tint for the control `id`.
#[must_use]
pub fn window_control_press_progress(id: GlassId) -> Option<f32> {
    window_control_slot_index(id).map(|slot| snapshot().slot(slot).press)
}

/// Press scale for the control `id`.
#[must_use]
pub fn window_control_scale(id: GlassId) -> f32 {
    window_control_slot_index(id).map_or(PRESS_SCALE_REST, |slot| snapshot().slot(slot).scale)
}

/// Interaction values for the control `id`.
#[must_use]
pub fn window_control_interaction(id: GlassId) -> TrafficLightInteraction {
    snapshot().interaction(id)
}

/// Flat slot index (`group * 3 + within`) for a raw control id.
///
/// Version-free: a consumer whose glass types come from a different
/// `liquid-glass-scene` release can still address the shared interaction frame.
#[must_use]
pub fn slot_index_for_raw_id(id: u64) -> Option<usize> {
    let group = group_index_for_raw_id(id)?;
    let within = WINDOW_CONTROL_RAW_GROUPS[group].iter().position(|item| *item == id)?;
    Some(group * GROUP_LEN + within)
}

/// Sample-group index owning a raw control id.
#[must_use]
pub fn group_index_for_raw_id(id: u64) -> Option<usize> {
    WINDOW_CONTROL_RAW_GROUPS.iter().position(|group| group.contains(&id))
}

/// Flat slot index (`group * 3 + within`) for a control id.
#[must_use]
pub fn window_control_slot_index(id: GlassId) -> Option<usize> {
    slot_index_for_raw_id(id.0)
}

/// Sample-group index owning a control id.
#[must_use]
pub fn window_control_group_index(id: GlassId) -> Option<usize> {
    group_index_for_raw_id(id.0)
}

/// The three ids of a sample group.
#[must_use]
pub fn window_control_group_ids(group_index: usize) -> Option<[GlassId; 3]> {
    WINDOW_CONTROL_GROUPS.get(group_index).map(|(_, ids)| *ids)
}

/// Every known control id, in slot order.
#[must_use]
pub fn all_window_control_ids() -> [GlassId; SLOT_COUNT] {
    let mut ids = [GlassId(0); SLOT_COUNT];
    for (group_index, (_, group)) in WINDOW_CONTROL_GROUPS.iter().enumerate() {
        for (within, id) in group.iter().enumerate() {
            ids[group_index * GROUP_LEN + within] = *id;
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{
        WINDOW_CONTROL_DISABLED_IDS, WINDOW_CONTROL_INACTIVE_IDS, WINDOW_CONTROL_LARGE_IDS,
        WINDOW_CONTROL_NATIVE_IDS, WINDOW_CONTROL_REFERENCE_IDS,
    };

    #[test]
    fn slot_and_group_indices_round_trip() {
        for (group_index, (_, ids)) in WINDOW_CONTROL_GROUPS.iter().enumerate() {
            for (within, id) in ids.iter().enumerate() {
                assert_eq!(window_control_group_index(*id), Some(group_index));
                assert_eq!(
                    window_control_slot_index(*id),
                    Some(group_index * GROUP_LEN + within)
                );
            }
        }
        assert_eq!(window_control_group_index(GlassId(999)), None);
        assert_eq!(window_control_slot_index(GlassId(999)), None);
    }

    #[test]
    fn raw_ids_are_the_single_source_for_the_typed_ids() {
        for (group_index, raw) in crate::layout::WINDOW_CONTROL_RAW_GROUPS.iter().enumerate() {
            let typed = WINDOW_CONTROL_GROUPS[group_index].1;
            for (within, raw_id) in raw.iter().enumerate() {
                assert_eq!(typed[within].0, *raw_id, "typed id drifted from the raw table");
                assert_eq!(slot_index_for_raw_id(*raw_id), Some(group_index * GROUP_LEN + within));
            }
        }
        // Every id is unique across the whole laboratory.
        let mut ids: Vec<u64> =
            crate::layout::WINDOW_CONTROL_RAW_GROUPS.iter().flatten().copied().collect();
        ids.sort_unstable();
        let unique = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), unique, "sample ids must be unique");
    }

    #[test]
    fn sample_groups_are_the_documented_five() {
        assert_eq!(WINDOW_CONTROL_NATIVE_IDS, WINDOW_CONTROL_GROUPS[0].1);
        assert_eq!(WINDOW_CONTROL_REFERENCE_IDS, WINDOW_CONTROL_GROUPS[1].1);
        assert_eq!(WINDOW_CONTROL_LARGE_IDS, WINDOW_CONTROL_GROUPS[2].1);
        assert_eq!(WINDOW_CONTROL_INACTIVE_IDS, WINDOW_CONTROL_GROUPS[3].1);
        assert_eq!(WINDOW_CONTROL_DISABLED_IDS, WINDOW_CONTROL_GROUPS[4].1);
        assert_eq!(all_window_control_ids().len(), SLOT_COUNT);
    }

    #[test]
    fn publishing_a_state_is_atomic_per_group() {
        let mut state = TrafficLightsState::new();
        state.on_group_hover(true);
        state.on_press_start(1);

        let mut frame = TrafficLightsFrame::default();
        frame.publish(2, &state);

        for value in frame.groups[2] {
            assert_eq!(value.hover, state.hover_progress);
        }
        assert_eq!(frame.slot(2 * GROUP_LEN + 1).press, state.press_tint(1));
        assert_eq!(frame.slot(2 * GROUP_LEN + 1).scale, state.press_scale(1));
        assert_eq!(frame.groups[0], [TrafficLightInteraction::REST; GROUP_LEN]);
    }
}
