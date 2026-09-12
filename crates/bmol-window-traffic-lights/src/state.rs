//! The traffic-light interaction state machine.
//!
//! This module is the **single source of truth** for traffic-light interaction.
//! It never mentions Iced, the GPU, or a global, so its behaviour is fully
//! assertable with ordinary unit tests.
//!
//! # Press scale contract
//!
//! A control only ever rests at [`PRESS_SCALE_REST`] or is held at
//! [`PRESS_SCALE_PEAK`]:
//!
//! | event | tint | scale target | `armed` |
//! |-------|------|--------------|---------|
//! | [`TrafficLightsEvent::PressStart`] | on | [`PRESS_SCALE_PEAK`] | `Some(index)` |
//! | [`TrafficLightsEvent::PressCancel`] (pointer dragged off while down) | off | unchanged — the control **stays enlarged until the mouse button is released** | unchanged |
//! | [`TrafficLightsEvent::PressEnd`] (released, inside or outside) | off | [`PRESS_SCALE_REST`] | `None` |
//!
//! Release is unconditional: any `PressEnd` settles the control and clears
//! `armed`, so a release can never be lost even if the widget tree was rebuilt
//! mid-press. [`TrafficLightsState::release_armed`] is the same safety net for
//! the case where the release event never arrives at all (focus loss, window
//! closed under the pointer).

use std::time::Instant;

use spring_rs::{Spring, SpringMotion};

use crate::action::{WindowControlAction, WindowExpandBehavior};

/// Resting scale of a traffic-light sphere.
pub const PRESS_SCALE_REST: f32 = 1.0;
/// Scale a traffic-light sphere reaches while the pointer holds it down.
///
/// This is the spring *target*, not the trajectory maximum: the spring carries
/// a slight bounce that lands a hair above it (see [`PRESS_SPRING_BOUNCE`]).
pub const PRESS_SCALE_PEAK: f32 = 1.18;
/// Perceptual duration of the press spring, in seconds.
pub const PRESS_SPRING_DURATION: f32 = 0.28;
/// Perceptual bounce of the press spring (`0.0` = critically damped).
///
/// `0.70` maps to `ζ = 0.30`, which overshoots the target by ~37% of the
/// travel: a distinctly bouncy settle, peaking around `1.25` and dipping to
/// roughly `0.93` when released.
pub const PRESS_SPRING_BOUNCE: f32 = 0.70;
/// Time constant used while revealing the group glyphs.
pub const INTERACTION_ENTER_ANIMATION_TIME_CONSTANT: f32 = 0.08;
/// Time constant used while hiding the group glyphs.
pub const INTERACTION_EXIT_ANIMATION_TIME_CONSTANT: f32 = 0.18;

/// Longest frame delta the state machine integrates in one step.
const MAX_STEP: f32 = 0.1;
/// Delta assumed when the state machine has no previous tick to measure from.
const NOMINAL_FRAME: f32 = 1.0 / 60.0;
/// Value at (or below) which a transition snaps to its target.
const SNAP_EPSILON: f32 = 0.0005;

/// Per-control interaction state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrafficLightSlot {
    /// Press tint, `0..=1`. Drawn by the glyph layer and the compositor.
    pub press: f32,
    /// Where [`Self::press`] is heading (`0.0` or `1.0`).
    pub press_target: f32,
    /// Scale spring. Its target is only ever
    /// [`PRESS_SCALE_REST`] or [`PRESS_SCALE_PEAK`].
    pub scale: SpringMotion,
}

impl TrafficLightSlot {
    fn new() -> Self {
        Self {
            press: 0.0,
            press_target: 0.0,
            scale: SpringMotion::new(
                PRESS_SCALE_REST,
                PRESS_SCALE_REST,
                Spring::perceptual(PRESS_SPRING_DURATION, PRESS_SPRING_BOUNCE),
            ),
        }
    }
}

/// Unified interaction event emitted by the traffic-light widget.
///
/// The widget reports *facts*: what the pointer did. The state machine decides
/// what they mean, and returns a [`WindowControlAction`] only when a press was
/// committed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrafficLightsEvent {
    /// The pointer entered or left the whole group. Drives the glyph reveal.
    GroupHover(bool),
    /// The pointer pressed this control.
    PressStart(usize),
    /// The pointer left this control while still holding the button.
    PressCancel(usize),
    /// The button was released. `committed` is `true` when the pointer was over
    /// the same control it pressed.
    PressEnd { index: usize, committed: bool },
}

impl TrafficLightsEvent {
    /// The control index this event addresses, if any.
    #[must_use]
    pub const fn index(self) -> Option<usize> {
        match self {
            Self::GroupHover(_) => None,
            Self::PressStart(index)
            | Self::PressCancel(index)
            | Self::PressEnd { index, .. } => Some(index),
        }
    }
}

/// Dynamic interaction and animation state for a traffic-light group.
///
/// One instance owns exactly one group of three controls. A laboratory that
/// shows several groups owns one state per group instead of parallel arrays.
#[derive(Debug, Clone, PartialEq)]
pub struct TrafficLightsState {
    /// Group glyph reveal, `0..=1`. macOS reveals all three symbols together.
    pub hover_progress: f32,
    /// Where [`Self::hover_progress`] is heading.
    pub hover_target: f32,
    /// The three controls, in Apple's close/minimize/zoom order.
    pub slots: [TrafficLightSlot; 3],
    /// The control the pointer is currently holding, if any.
    pub armed: Option<usize>,
    /// What the green control does when it commits.
    pub expand_behavior: WindowExpandBehavior,
    last_tick: Option<Instant>,
}

impl Default for TrafficLightsState {
    fn default() -> Self {
        Self {
            hover_progress: 0.0,
            hover_target: 0.0,
            slots: [TrafficLightSlot::new(); 3],
            armed: None,
            expand_behavior: WindowExpandBehavior::Fullscreen,
            last_tick: None,
        }
    }
}

impl TrafficLightsState {
    /// Creates a settled group at rest.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a settled group with a specific green-control behaviour.
    #[must_use]
    pub fn with_expand_behavior(expand_behavior: WindowExpandBehavior) -> Self {
        Self { expand_behavior, ..Self::default() }
    }

    /// The action a committed press on `index` represents.
    #[must_use]
    pub const fn action_for(&self, index: usize) -> WindowControlAction {
        match index {
            0 => WindowControlAction::Close,
            1 => WindowControlAction::Minimize,
            _ => {
                if self.expand_behavior.is_fullscreen() {
                    WindowControlAction::Expand
                } else {
                    WindowControlAction::Zoom
                }
            }
        }
    }

    /// Marks the group as hovered or not, and wakes the animation clock.
    pub fn on_group_hover(&mut self, hovered: bool) {
        self.hover_target = if hovered { 1.0 } else { 0.0 };
        self.wake();
    }

    /// Arms this control: tint on, scale target at [`PRESS_SCALE_PEAK`].
    pub fn on_press_start(&mut self, index: usize) {
        if index < self.slots.len() {
            let slot = &mut self.slots[index];
            slot.press_target = 1.0;
            slot.scale.retarget(PRESS_SCALE_PEAK);
            self.armed = Some(index);
            self.wake();
        }
    }

    /// The pointer left the control while still holding.
    ///
    /// Per the agreed contract the tint clears but the control **keeps its
    /// enlarged scale until the button is released**; re-entering restores the
    /// tint without replaying the growth bounce, because the spring target
    /// never changed.
    pub fn on_press_cancel(&mut self, index: usize) {
        if let Some(slot) = self.slots.get_mut(index) {
            slot.press_target = 0.0;
        }
    }

    /// The button was released. Always settles, in every circumstance.
    pub fn on_press_end(&mut self, index: usize) {
        if let Some(slot) = self.slots.get_mut(index) {
            slot.press_target = 0.0;
            slot.scale.retarget(PRESS_SCALE_REST);
        }
        if self.armed == Some(index) {
            self.armed = None;
        }
        self.wake();
    }

    /// Safety net for a release that never reached the widget: settles whichever
    /// control is armed, without committing anything.
    pub fn release_armed(&mut self) {
        if let Some(index) = self.armed.take() {
            if let Some(slot) = self.slots.get_mut(index) {
                slot.press_target = 0.0;
                slot.scale.retarget(PRESS_SCALE_REST);
            }
            self.wake();
        }
    }

    /// Settles every control and clears the hover reveal. Used when the whole
    /// pointer interaction is torn down (window blur, layout change).
    pub fn reset_interaction(&mut self) {
        self.armed = None;
        self.hover_target = 0.0;
        for slot in &mut self.slots {
            slot.press_target = 0.0;
            slot.scale.retarget(PRESS_SCALE_REST);
        }
        self.wake();
    }

    /// Current scale of control `index`, for the glyph layer and the compositor.
    #[must_use]
    pub fn press_scale(&self, index: usize) -> f32 {
        self.slots.get(index).map_or(PRESS_SCALE_REST, |slot| slot.scale.value())
    }

    /// Current press tint of control `index`.
    #[must_use]
    pub fn press_tint(&self, index: usize) -> f32 {
        self.slots.get(index).map_or(0.0, |slot| slot.press)
    }

    /// Advances the simulation by an explicit delta, in seconds.
    ///
    /// Exposed separately from [`Self::step`] so tests can integrate with a
    /// deterministic frame clock instead of the wall clock.
    pub fn advance(&mut self, dt: f32) {
        let dt = if dt.is_finite() { dt.clamp(0.0, MAX_STEP) } else { 0.0 };

        let hover_tc = if self.hover_target >= self.hover_progress {
            INTERACTION_ENTER_ANIMATION_TIME_CONSTANT
        } else {
            INTERACTION_EXIT_ANIMATION_TIME_CONSTANT
        };
        if dt > 0.0 {
            let hover_step = 1.0 - (-dt / hover_tc).exp();
            self.hover_progress += (self.hover_target - self.hover_progress) * hover_step;
        }
        if (self.hover_target - self.hover_progress).abs() < SNAP_EPSILON {
            self.hover_progress = self.hover_target;
        }

        for slot in &mut self.slots {
            let press_tc = if slot.press_target >= slot.press {
                INTERACTION_ENTER_ANIMATION_TIME_CONSTANT
            } else {
                INTERACTION_EXIT_ANIMATION_TIME_CONSTANT
            };
            if dt > 0.0 {
                let press_step = 1.0 - (-dt / press_tc).exp();
                slot.press += (slot.press_target - slot.press) * press_step;
            }
            if (slot.press_target - slot.press).abs() < SNAP_EPSILON {
                slot.press = slot.press_target;
            }
            if dt > 0.0 {
                slot.scale.step(dt);
            }
        }

        if !self.is_animating() {
            self.last_tick = None;
        } else {
            // Keep the clock fresh while work remains.
            self.last_tick.get_or_insert_with(Instant::now);
        }
    }

    /// Advances the simulation from the wall clock.
    pub fn step(&mut self, now: Instant) {
        let dt = self
            .last_tick
            .map_or(NOMINAL_FRAME, |last| now.saturating_duration_since(last).as_secs_f32());
        self.last_tick = Some(now);
        self.advance(dt);
    }

    /// Returns `true` while any value still needs frames to reach its target.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        (self.hover_target - self.hover_progress).abs() > SNAP_EPSILON
            || self.slots.iter().any(|slot| {
                (slot.press_target - slot.press).abs() > SNAP_EPSILON
                    || !slot.scale.is_settled(SNAP_EPSILON, 0.01)
            })
    }

    /// Consumes an interaction event, updating physics and hover state.
    ///
    /// Returns `Some(action)` when a press was committed to; the caller decides
    /// whether that action should reach the window.
    pub fn handle_event(&mut self, event: TrafficLightsEvent) -> Option<WindowControlAction> {
        match event {
            TrafficLightsEvent::GroupHover(hovered) => {
                self.on_group_hover(hovered);
                None
            }
            TrafficLightsEvent::PressStart(index) => {
                self.on_press_start(index);
                None
            }
            TrafficLightsEvent::PressCancel(index) => {
                self.on_press_cancel(index);
                None
            }
            TrafficLightsEvent::PressEnd { index, committed } => {
                self.on_press_end(index);
                committed.then(|| self.action_for(index))
            }
        }
    }

    fn wake(&mut self) {
        self.last_tick.get_or_insert_with(Instant::now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f32 = 1.0 / 60.0;

    /// Integrates until settled, returning the trajectory extremes.
    fn settle(state: &mut TrafficLightsState, index: usize, max_frames: usize) -> (f32, f32) {
        let mut peak = state.press_scale(index);
        let mut trough = state.press_scale(index);
        for _ in 0..max_frames {
            state.advance(FRAME);
            let scale = state.press_scale(index);
            peak = peak.max(scale);
            trough = trough.min(scale);
            if !state.is_animating() {
                break;
            }
        }
        (peak, trough)
    }

    #[test]
    fn press_peaks_slightly_above_the_target_and_returns_to_rest() {
        let mut state = TrafficLightsState::new();
        state.on_press_start(0);
        let (peak, _) = settle(&mut state, 0, 600);

        // The spring target is the peak; the bounce lands just above it.
        assert!(peak >= PRESS_SCALE_PEAK, "peak {peak} should reach the target");
        // The spring is meant to be elastic: it must overshoot the target
        // visibly, without the wide bounce the previous preset produced.
        assert!(peak > 1.22, "peak {peak} should overshoot for elasticity");
        assert!(peak <= 1.27, "peak {peak} should stay close to the intent");
        assert!((state.press_scale(0) - PRESS_SCALE_PEAK).abs() < 1e-3);

        state.on_press_end(0);
        let (_, trough) = settle(&mut state, 0, 600);

        assert!((state.press_scale(0) - PRESS_SCALE_REST).abs() < 1e-3);
        // A bouncy release is expected to undershoot before settling.
        assert!(trough > 0.90, "release undershoot is too deep: {trough}");
        assert!(trough < 1.0, "release should visibly spring back: {trough}");
        assert_eq!(state.armed, None);
    }

    #[test]
    fn step_advances_from_the_wall_clock() {
        // The application drives the state machine through `step(Instant)`, not
        // `advance(dt)`. Cover that path explicitly: it is the one the demo
        // actually runs.
        let mut state = TrafficLightsState::new();
        let t0 = Instant::now();
        state.step(t0);
        state.on_press_start(0);

        state.step(t0 + std::time::Duration::from_millis(16));
        let after_one = state.press_scale(0);
        assert!(after_one > 1.0, "wall-clock step did not move the spring: {after_one}");

        state.step(t0 + std::time::Duration::from_millis(32));
        assert!(state.press_scale(0) > after_one, "spring did not keep advancing");
    }

    #[test]
    fn cancel_keeps_the_control_enlarged_until_release() {
        let mut state = TrafficLightsState::new();
        state.on_press_start(1);
        settle(&mut state, 1, 600);
        let enlarged = state.press_scale(1);
        assert!(enlarged > 1.17);

        state.on_press_cancel(1);
        assert_eq!(state.armed, Some(1));
        for _ in 0..120 {
            state.advance(FRAME);
        }
        assert_eq!(state.press_tint(1), 0.0, "drag-off must clear the tint");
        assert!(
            (state.press_scale(1) - enlarged).abs() < 1e-3,
            "drag-off must keep the scale until release"
        );

        // Re-entering must not replay the growth bounce.
        state.on_press_start(1);
        assert!((state.press_scale(1) - enlarged).abs() < 1e-3);

        state.on_press_end(1);
        settle(&mut state, 1, 600);
        assert!((state.press_scale(1) - PRESS_SCALE_REST).abs() < 1e-3);
    }

    #[test]
    fn release_armed_is_a_safe_fallback() {
        let mut state = TrafficLightsState::new();
        state.on_press_start(2);
        settle(&mut state, 2, 600);
        state.release_armed();
        assert_eq!(state.armed, None);
        settle(&mut state, 2, 600);
        assert!((state.press_scale(2) - PRESS_SCALE_REST).abs() < 1e-3);
    }

    #[test]
    fn step_is_frame_rate_independent() {
        let mut coarse = TrafficLightsState::new();
        coarse.on_group_hover(true);
        coarse.advance(0.1);

        let mut fine = TrafficLightsState::new();
        fine.on_group_hover(true);
        for _ in 0..10 {
            fine.advance(0.01);
        }

        assert!(
            (coarse.hover_progress - fine.hover_progress).abs() < 0.02,
            "{} vs {}",
            coarse.hover_progress,
            fine.hover_progress
        );
    }

    #[test]
    fn handle_event_round_trip() {
        let mut state = TrafficLightsState::with_expand_behavior(WindowExpandBehavior::Maximize);

        assert_eq!(state.handle_event(TrafficLightsEvent::GroupHover(true)), None);
        assert_eq!(state.hover_target, 1.0);
        assert!(state.is_animating());

        assert_eq!(state.handle_event(TrafficLightsEvent::PressStart(0)), None);
        assert_eq!(state.armed, Some(0));

        // Releasing off the control settles it but commits nothing.
        assert_eq!(
            state.handle_event(TrafficLightsEvent::PressEnd { index: 0, committed: false }),
            None
        );
        assert_eq!(state.armed, None);

        // Releasing on the control commits the action for that index.
        state.handle_event(TrafficLightsEvent::PressStart(2));
        assert_eq!(
            state.handle_event(TrafficLightsEvent::PressEnd { index: 2, committed: true }),
            Some(WindowControlAction::Zoom)
        );
        assert_eq!(state.action_for(0), WindowControlAction::Close);
        assert_eq!(state.action_for(1), WindowControlAction::Minimize);
    }

    #[test]
    fn event_index_addresses_the_control() {
        assert_eq!(TrafficLightsEvent::GroupHover(true).index(), None);
        assert_eq!(TrafficLightsEvent::PressStart(2).index(), Some(2));
        assert_eq!(TrafficLightsEvent::PressCancel(1).index(), Some(1));
        assert_eq!(
            TrafficLightsEvent::PressEnd { index: 2, committed: true }.index(),
            Some(2)
        );
    }

    #[test]
    fn fullscreen_and_maximize_pick_distinct_green_actions() {
        assert_eq!(
            TrafficLightsState::with_expand_behavior(WindowExpandBehavior::Fullscreen).action_for(2),
            WindowControlAction::Expand
        );
        assert_eq!(
            TrafficLightsState::with_expand_behavior(WindowExpandBehavior::Maximize).action_for(2),
            WindowControlAction::Zoom
        );
    }

    #[test]
    fn reset_interaction_settles_everything() {
        let mut state = TrafficLightsState::new();
        state.on_group_hover(true);
        state.on_press_start(0);
        settle(&mut state, 0, 600);

        state.reset_interaction();
        settle(&mut state, 0, 600);

        assert_eq!(state.armed, None);
        assert_eq!(state.hover_progress, 0.0);
        assert_eq!(state.hover_target, 0.0);
        assert!((state.press_scale(0) - PRESS_SCALE_REST).abs() < 1e-3);
    }
}
