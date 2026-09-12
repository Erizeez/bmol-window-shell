//! Standalone Iced laboratory for custom macOS-style window controls.
//!
//! The demo deliberately hides the native window decorations and renders the
//! traffic lights through the same Liquid Glass compositor used by the
//! settings demo. It shows the measured 14 pt controls, a second 1:1 sample,
//! and enlarged active/inactive samples for inspecting hover, press, and
//! release light.
//!
//! # Interaction model
//!
//! Every sample group owns exactly one [`TrafficLightsState`]. The widget emits
//! [`TrafficLightsEvent`] facts; the state machine turns them into hover,
//! press, and scale values; `iced_backend::publish_group` hands the resulting
//! frame to the compositor. There are no parallel animation arrays here, and
//! "animates" is no longer conflated with "executes a window command": only the
//! measured sample drives the real window.

#[path = "playground/iced_backend.rs"]
mod iced_backend;

use std::time::{Duration, Instant};

use iced::{
    Background, Color, Element, Length, Padding, Subscription, Task, Theme,
    widget::{button, column, container, row, scrollable, space, stack, text},
};
use iced_backend::{DemoSurface, Renderer, WindowControlTuning};
use liquid_glass::{
    GlassId, IcedWindowController, IcedWindowPolicy, UiColorScheme, UiCornerStyle, UiTheme,
    WindowCommand, WindowDragArea,
    ui::{components, font},
};

pub use bmol_window_shell::traffic_lights as window_controls;

pub use window_controls::{
    ControlAction, ControlGroup, ControlGroupStyle, TrafficLightsEvent, TrafficLightsState,
    WINDOW_CONTROL_DISABLED_IDS, WINDOW_CONTROL_DISABLED_X, WINDOW_CONTROL_DISABLED_Y,
    WINDOW_CONTROL_GAP, WINDOW_CONTROL_INACTIVE_IDS, WINDOW_CONTROL_INACTIVE_X,
    WINDOW_CONTROL_INACTIVE_Y, WINDOW_CONTROL_LARGE_GAP, WINDOW_CONTROL_LARGE_IDS,
    WINDOW_CONTROL_LARGE_SIZE, WINDOW_CONTROL_LARGE_X, WINDOW_CONTROL_LARGE_Y,
    WINDOW_CONTROL_NATIVE_IDS, WINDOW_CONTROL_NATIVE_SIZE, WINDOW_CONTROL_NATIVE_X,
    WINDOW_CONTROL_NATIVE_Y, WINDOW_CONTROL_REFERENCE_IDS, WINDOW_CONTROL_REFERENCE_X,
    WINDOW_CONTROL_REFERENCE_Y, WindowExpandBehavior, window_control_group,
};

type AppElement<'a> = Element<'a, Message, Theme, Renderer>;

/// One traffic-light sample in the laboratory.
#[derive(Clone, Copy)]
struct Sample {
    group: ControlGroup,
    ids: [GlassId; 3],
    x: f32,
    y: f32,
    size: f32,
    gap: f32,
    label: &'static str,
    /// Renders the unfocused-window treatment.
    inactive_window: bool,
    /// The close control is unavailable.
    close_disabled: bool,
    /// The close control shows the unsaved-document dot.
    close_dot: bool,
    expand: WindowExpandBehavior,
    /// Whether a committed click reaches the real window.
    executes: bool,
}

const SAMPLES: [Sample; 5] = [
    Sample {
        group: ControlGroup::Native,
        ids: WINDOW_CONTROL_NATIVE_IDS,
        x: WINDOW_CONTROL_NATIVE_X,
        y: WINDOW_CONTROL_NATIVE_Y,
        size: WINDOW_CONTROL_NATIVE_SIZE,
        gap: WINDOW_CONTROL_GAP,
        label: "Measured control · drives the real window",
        inactive_window: false,
        close_disabled: false,
        close_dot: false,
        expand: WindowExpandBehavior::Fullscreen,
        executes: true,
    },
    Sample {
        group: ControlGroup::Reference,
        ids: WINDOW_CONTROL_REFERENCE_IDS,
        x: WINDOW_CONTROL_REFERENCE_X,
        y: WINDOW_CONTROL_REFERENCE_Y,
        size: WINDOW_CONTROL_NATIVE_SIZE,
        gap: WINDOW_CONTROL_GAP,
        label: "1:1 reference · 14 pt visual diameter",
        inactive_window: false,
        close_disabled: false,
        close_dot: false,
        expand: WindowExpandBehavior::Fullscreen,
        executes: false,
    },
    Sample {
        group: ControlGroup::Active,
        ids: WINDOW_CONTROL_LARGE_IDS,
        x: WINDOW_CONTROL_LARGE_X,
        y: WINDOW_CONTROL_LARGE_Y,
        size: WINDOW_CONTROL_LARGE_SIZE,
        gap: WINDOW_CONTROL_LARGE_GAP,
        label: "Maximize behavior · plus glyph · 64 pt",
        inactive_window: false,
        close_disabled: false,
        close_dot: false,
        expand: WindowExpandBehavior::Maximize,
        executes: false,
    },
    Sample {
        group: ControlGroup::Inactive,
        ids: WINDOW_CONTROL_INACTIVE_IDS,
        x: WINDOW_CONTROL_INACTIVE_X,
        y: WINDOW_CONTROL_INACTIVE_Y,
        size: WINDOW_CONTROL_LARGE_SIZE,
        gap: WINDOW_CONTROL_LARGE_GAP,
        label: "Inactive window · all controls pale gray",
        inactive_window: true,
        close_disabled: false,
        close_dot: false,
        expand: WindowExpandBehavior::Fullscreen,
        executes: false,
    },
    Sample {
        group: ControlGroup::Disabled,
        ids: WINDOW_CONTROL_DISABLED_IDS,
        x: WINDOW_CONTROL_DISABLED_X,
        y: WINDOW_CONTROL_DISABLED_Y,
        size: WINDOW_CONTROL_LARGE_SIZE,
        gap: WINDOW_CONTROL_LARGE_GAP,
        label: "Running · close status dot",
        inactive_window: false,
        close_disabled: true,
        close_dot: true,
        expand: WindowExpandBehavior::Maximize,
        executes: false,
    },
];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum TuningParameter {
    BlurRadius,
    InternalScattering,
    SideEdgeDarkness,
    SideEdgeWidth,
    Opacity,
    SubstrateCoverage,
    LowerSubstrateCoverage,
    LowerTintCoverage,
    AngularLight,
    LightAngle,
    LightSoftness,
    BodyThickness,
    EdgeSideBias,
    EdgeSideAngle,
    RefractionStrength,
    FresnelStrength,
    HoverGain,
    PressGain,
    PressLift,
    UniformLight,
    ThinLightGain,
}

#[derive(Debug, Clone)]
enum Message {
    /// One interaction fact from a sample group.
    Lights {
        group: ControlGroup,
        event: TrafficLightsEvent,
    },
    AnimationTick(Instant),
    WindowReady(Option<iced::window::Id>),
    WindowEvent((iced::window::Id, iced::window::Event)),
    BeginWindowDrag,
    TuningChanged {
        parameter: TuningParameter,
        value: f32,
    },
    CopyConfiguration,
    ToggleScheme,
    ToggleDocumentEdited,
    SystemThemeChanged(iced::theme::Mode),
}

struct State {
    window: IcedWindowController,
    window_policy: IcedWindowPolicy,
    scheme: UiColorScheme,
    focused: bool,
    document_edited: bool,
    /// One state machine per sample group, indexed by [`ControlGroup::index`].
    lights: [TrafficLightsState; SAMPLES.len()],
    last_action: String,
    tuning: WindowControlTuning,
}

impl Default for State {
    fn default() -> Self {
        Self {
            window: IcedWindowController::new(),
            window_policy: demo_window_policy(),
            scheme: UiColorScheme::Light,
            focused: true,
            document_edited: false,
            lights: [
                TrafficLightsState::new(),
                TrafficLightsState::new(),
                TrafficLightsState::new(),
                TrafficLightsState::new(),
                TrafficLightsState::new(),
            ],
            last_action: "Move over a control to reveal its system glyph".into(),
            tuning: WindowControlTuning::default(),
        }
    }
}

fn boot() -> (State, Task<Message>) {
    let state = State::default();
    iced_backend::set_surface(DemoSurface::WindowControls);
    iced_backend::set_color_scheme(state.scheme);
    iced_backend::set_accessibility(liquid_glass::GlassAccessibility::none());
    iced_backend::set_window_control_tuning(state.tuning);
    iced_backend::reset_groups();
    (
        state,
        Task::batch([
            iced::system::theme().map(Message::SystemThemeChanged),
            IcedWindowController::latest().map(Message::WindowReady),
        ]),
    )
}

const fn demo_window_policy() -> IcedWindowPolicy {
    IcedWindowPolicy::liquid_glass().manual_close()
}

/// Human-readable status line for one interaction fact.
fn describe(event: TrafficLightsEvent) -> String {
    match event {
        TrafficLightsEvent::GroupHover(true) => "Hover — system glyphs revealed".into(),
        TrafficLightsEvent::GroupHover(false) => "Pointer left the controls".into(),
        TrafficLightsEvent::PressStart(index) => format!("{} pressed", action_name(index)),
        TrafficLightsEvent::PressCancel(index) => {
            format!("{} press cancelled — drag back to re-arm", action_name(index))
        }
        TrafficLightsEvent::PressEnd {
            index,
            committed: true,
        } => format!("{} released", action_name(index)),
        TrafficLightsEvent::PressEnd {
            index,
            committed: false,
        } => format!("{} released outside — no action", action_name(index)),
    }
}

const fn action_name(index: usize) -> &'static str {
    match index {
        0 => "Close",
        1 => "Minimize",
        _ => "Zoom",
    }
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Lights { group, event } => {
            let slot = group.index();
            let action = state.lights[slot].handle_event(event);
            iced_backend::publish_group(slot, &state.lights[slot]);
            state.last_action = describe(event);

            if let Some(action) = action {
                if SAMPLES[slot].executes {
                    state.last_action =
                        format!("{} control committed — sent to the Iced window", action.name());
                    let command = match action {
                        ControlAction::Close => WindowCommand::Close,
                        ControlAction::Minimize => WindowCommand::Minimize,
                        ControlAction::Expand | ControlAction::Zoom => {
                            state.window_policy.expand_command()
                        }
                    };
                    return state.window.task(command);
                }
                state.last_action =
                    format!("{} control committed — inspection sample", action.name());
            }
        }
        Message::AnimationTick(now) => {
            for lights in &mut state.lights {
                lights.step(now);
            }
            iced_backend::publish_groups(&state.lights);
        }
        Message::WindowReady(id) => {
            if let Some(id) = id {
                state.window.attach(id);
            }
        }
        Message::WindowEvent((id, event)) => {
            let close_requested = matches!(event, iced::window::Event::CloseRequested);
            let focused = match event {
                iced::window::Event::Focused => Some(true),
                iced::window::Event::Unfocused => Some(false),
                _ => None,
            };
            state.window.observe(id, &event);
            if let Some(focused) = focused {
                state.focused = focused;
            }
            if close_requested {
                // This demo explicitly uses `manual_close` so a real app can
                // insert confirmation or save logic here. It has nothing to
                // save, so it closes immediately.
                return state.window.task(WindowCommand::Close);
            }
        }
        Message::BeginWindowDrag => {
            return state.window.task(WindowCommand::BeginDrag);
        }
        Message::TuningChanged { parameter, value } => {
            match parameter {
                TuningParameter::BlurRadius => state.tuning.blur_radius = value,
                TuningParameter::InternalScattering => state.tuning.internal_scattering = value,
                TuningParameter::SideEdgeDarkness => state.tuning.side_edge_darkness = value,
                TuningParameter::SideEdgeWidth => state.tuning.side_edge_width = value,
                TuningParameter::Opacity => state.tuning.opacity = value,
                TuningParameter::SubstrateCoverage => state.tuning.substrate_coverage = value,
                TuningParameter::LowerSubstrateCoverage => {
                    state.tuning.lower_substrate_coverage = value;
                }
                TuningParameter::LowerTintCoverage => state.tuning.lower_tint_coverage = value,
                TuningParameter::AngularLight => state.tuning.angular_light = value,
                TuningParameter::LightAngle => state.tuning.light_angle = value,
                TuningParameter::LightSoftness => state.tuning.light_softness = value,
                TuningParameter::BodyThickness => state.tuning.body_thickness = value,
                TuningParameter::EdgeSideBias => state.tuning.edge_side_bias = value,
                TuningParameter::EdgeSideAngle => state.tuning.edge_side_angle = value,
                TuningParameter::RefractionStrength => {
                    state.tuning.refraction_strength = value;
                }
                TuningParameter::FresnelStrength => state.tuning.fresnel_strength = value,
                TuningParameter::HoverGain => state.tuning.hover_gain = value,
                TuningParameter::PressGain => state.tuning.press_gain = value,
                TuningParameter::PressLift => state.tuning.press_lift = value,
                TuningParameter::UniformLight => state.tuning.uniform_light = value,
                TuningParameter::ThinLightGain => state.tuning.thin_light_gain = value,
            }
            iced_backend::set_window_control_tuning(state.tuning);
        }
        Message::CopyConfiguration => {
            state.last_action = "Configuration copied to clipboard".into();
            return iced::clipboard::write(configuration_text(state));
        }
        Message::ToggleScheme => {
            state.scheme = match state.scheme {
                UiColorScheme::Light => UiColorScheme::Dark,
                UiColorScheme::Dark => UiColorScheme::Light,
            };
            state.tuning = WindowControlTuning::for_scheme(state.scheme == UiColorScheme::Dark);
            iced_backend::set_color_scheme(state.scheme);
            iced_backend::set_window_control_tuning(state.tuning);
        }
        Message::ToggleDocumentEdited => {
            state.document_edited = !state.document_edited;
            state.last_action = if state.document_edited {
                "Document marked as edited — close control shows the status dot".into()
            } else {
                "Document marked as saved — close control shows the glyph".into()
            };
        }
        Message::SystemThemeChanged(mode) => {
            state.scheme = UiColorScheme::from_mode(mode);
            state.tuning = WindowControlTuning::for_scheme(state.scheme == UiColorScheme::Dark);
            iced_backend::set_color_scheme(state.scheme);
            iced_backend::set_window_control_tuning(state.tuning);
        }
    }
    Task::none()
}

fn subscription(state: &State) -> Subscription<Message> {
    let theme_changes = iced::system::theme_changes().map(Message::SystemThemeChanged);
    let window_events = IcedWindowController::events().map(Message::WindowEvent);

    if state.lights.iter().any(TrafficLightsState::is_animating) {
        Subscription::batch([
            theme_changes,
            window_events,
            iced::time::every(Duration::from_millis(16))
                .map(|_| Message::AnimationTick(Instant::now())),
        ])
    } else {
        Subscription::batch([theme_changes, window_events])
    }
}

fn app_theme(state: &State) -> Theme {
    UiTheme::new(state.scheme).iced_theme()
}

/// Interaction style for one sample, read from its own state machine.
fn sample_style(sample: Sample, state: &State) -> ControlGroupStyle {
    let lights = &state.lights[sample.group.index()];
    ControlGroupStyle {
        size: sample.size,
        gap: sample.gap,
        is_dark: state.scheme == UiColorScheme::Dark,
        // The measured sample follows the real window focus; the inactive
        // sample is deliberately unfocused.
        is_active: !sample.inactive_window
            && (sample.group != ControlGroup::Native || state.focused),
        close_disabled: sample.close_disabled,
        close_dot: sample.close_dot && state.document_edited,
        expand_behavior: sample.expand,
        hover: lights.hover_progress,
        press: [lights.press_tint(0), lights.press_tint(1), lights.press_tint(2)],
        scales: [lights.press_scale(0), lights.press_scale(1), lights.press_scale(2)],
    }
}

fn view(state: &State) -> AppElement<'_> {
    let info = container(
        column![
            row![
                text("Custom Window Controls")
                    .size(28.0)
                    .font(font::ui_font(iced::font::Weight::Semibold)),
                space().width(Length::Fill),
                button(text(match state.document_edited {
                    true => "Mark document saved",
                    false => "Mark document edited",
                }))
                .on_press(Message::ToggleDocumentEdited)
                .padding([7, 12]),
                button(text("Copy configuration"))
                    .on_press(Message::CopyConfiguration)
                    .padding([7, 12]),
                button(text(match state.scheme {
                    UiColorScheme::Light => "Switch to dark mode",
                    UiColorScheme::Dark => "Switch to light mode",
                }))
                .on_press(Message::ToggleScheme)
                .padding([7, 12]),
            ]
            .align_y(iced::Alignment::Center),
            text("Live SDF glass · macOS-extracted vector glyphs · hover, drag-off, and release")
                .size(font::size::BODY)
                .style(components::secondary_text),
            text(&state.last_action).size(font::size::CAPTION).style(components::tertiary_text),
        ]
        .spacing(8),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(Padding { top: 86.0, right: 48.0, bottom: 0.0, left: 48.0 })
    .style(transparent_surface);

    let mut layers: Vec<AppElement<'_>> =
        vec![stage_background(state.scheme), window_drag_region(), info.into()];
    layers.push(positioned(tuning_panel(state.tuning), 620.0, 82.0));

    for sample in SAMPLES {
        let slop = window_controls::control_hover_slop(sample.size);
        layers.push(positioned(sample_label(sample.label), sample.x, sample.y - 30.0));
        layers.push(positioned(
            window_control_group(sample.ids, sample_style(sample, state), move |_, event| {
                Message::Lights {
                    group: sample.group,
                    event,
                }
            }),
            sample.x - slop,
            sample.y - slop,
        ));
    }

    container(stack(layers))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(transparent_surface)
        .into()
}

fn configuration_text(state: &State) -> String {
    let scheme = match state.scheme {
        UiColorScheme::Light => "light",
        UiColorScheme::Dark => "dark",
    };
    let tuning = state.tuning;

    format!(
        "liquid-glass-window-controls-demo\n\
scheme = {scheme}\n\
document_edited = {edited}\n\
blur_radius = {blur_radius:.2}\n\
internal_scattering = {internal_scattering:.4}\n\
side_edge_darkness = {side_edge_darkness:.4}\n\
side_edge_width = {side_edge_width:.4}\n\
opacity = {opacity:.4}\n\
substrate_coverage = {substrate_coverage:.4}\n\
lower_substrate_coverage = {lower_substrate_coverage:.4}\n\
lower_tint_coverage = {lower_tint_coverage:.4}\n\
angular_light = {angular_light:.4}\n\
light_angle = {light_angle:.4}\n\
light_softness = {light_softness:.4}\n\
body_thickness = {body_thickness:.4}\n\
edge_side_bias = {edge_side_bias:.4}\n\
edge_side_angle = {edge_side_angle:.2}\n\
refraction_strength = {refraction_strength:.4}\n\
fresnel_strength = {fresnel_strength:.4}\n\
hover_gain = {hover_gain:.4}\n\
press_gain = {press_gain:.4}\n\
press_lift = {press_lift:.4}\n\
uniform_light = {uniform_light:.4}\n\
thin_light_gain = {thin_light_gain:.4}\n",
        edited = state.document_edited,
        blur_radius = tuning.blur_radius,
        internal_scattering = tuning.internal_scattering,
        side_edge_darkness = tuning.side_edge_darkness,
        side_edge_width = tuning.side_edge_width,
        opacity = tuning.opacity,
        substrate_coverage = tuning.substrate_coverage,
        lower_substrate_coverage = tuning.lower_substrate_coverage,
        lower_tint_coverage = tuning.lower_tint_coverage,
        angular_light = tuning.angular_light,
        light_angle = tuning.light_angle,
        light_softness = tuning.light_softness,
        body_thickness = tuning.body_thickness,
        edge_side_bias = tuning.edge_side_bias,
        edge_side_angle = tuning.edge_side_angle,
        refraction_strength = tuning.refraction_strength,
        fresnel_strength = tuning.fresnel_strength,
        hover_gain = tuning.hover_gain,
        press_gain = tuning.press_gain,
        press_lift = tuning.press_lift,
        uniform_light = tuning.uniform_light,
        thin_light_gain = tuning.thin_light_gain,
    )
}

fn tuning_panel(tuning: WindowControlTuning) -> AppElement<'static> {
    let rows = components::settings_group(vec![
        components::setting_slider_with_step(
            "Blur radius",
            format!("{:.0} px", tuning.blur_radius),
            tuning.blur_radius,
            0.0..=80.0,
            1.0,
            |value| Message::TuningChanged { parameter: TuningParameter::BlurRadius, value },
        ),
        components::setting_slider_with_step(
            "Internal scattering",
            format!("{:.0}%", tuning.internal_scattering * 100.0),
            tuning.internal_scattering,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged {
                parameter: TuningParameter::InternalScattering,
                value,
            },
        ),
        components::setting_slider_with_step(
            "Edge darkness",
            format!("{:.2}", tuning.side_edge_darkness),
            tuning.side_edge_darkness,
            0.0..=4.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::SideEdgeDarkness, value },
        ),
        components::setting_slider_with_step(
            "Edge width",
            format!("{:.2}×", tuning.side_edge_width),
            tuning.side_edge_width,
            0.5..=4.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::SideEdgeWidth, value },
        ),
        components::setting_slider_with_step(
            "Opacity",
            format!("{:.0}%", tuning.opacity * 100.0),
            tuning.opacity,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::Opacity, value },
        ),
        components::setting_slider_with_step(
            "Upper substrate",
            format!("{:.0}%", tuning.substrate_coverage * 100.0),
            tuning.substrate_coverage,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::SubstrateCoverage, value },
        ),
        components::setting_slider_with_step(
            "Lower substrate",
            format!("{:.0}%", tuning.lower_substrate_coverage * 100.0),
            tuning.lower_substrate_coverage,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged {
                parameter: TuningParameter::LowerSubstrateCoverage,
                value,
            },
        ),
        components::setting_slider_with_step(
            "Lower tint coverage",
            format!("{:.0}%", tuning.lower_tint_coverage * 100.0),
            tuning.lower_tint_coverage,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::LowerTintCoverage, value },
        ),
        components::setting_slider_with_step(
            "Angular light",
            format!("{:.1}%", tuning.angular_light * 100.0),
            tuning.angular_light,
            0.0..=0.15,
            0.001,
            |value| Message::TuningChanged { parameter: TuningParameter::AngularLight, value },
        ),
        components::setting_slider_with_step(
            "Light angle",
            format!("{:.0}°", tuning.light_angle * 90.0),
            tuning.light_angle,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::LightAngle, value },
        ),
        components::setting_slider_with_step(
            "Light softness",
            format!("{:.0}%", tuning.light_softness * 100.0),
            tuning.light_softness,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::LightSoftness, value },
        ),
        components::setting_slider_with_step(
            "Body thickness",
            format!("{:.2}×", tuning.body_thickness),
            tuning.body_thickness,
            0.4..=2.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::BodyThickness, value },
        ),
        components::setting_slider_with_step(
            "Edge side bias",
            format!("{:.0}%", tuning.edge_side_bias * 100.0),
            tuning.edge_side_bias,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::EdgeSideBias, value },
        ),
        components::setting_slider_with_step(
            "Side thickness sigma",
            format!("{:.0}° σ", tuning.edge_side_angle),
            tuning.edge_side_angle,
            10.0..=80.0,
            1.0,
            |value| Message::TuningChanged { parameter: TuningParameter::EdgeSideAngle, value },
        ),
        components::setting_slider_with_step(
            "Refraction",
            format!("{:.2}", tuning.refraction_strength),
            tuning.refraction_strength,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged {
                parameter: TuningParameter::RefractionStrength,
                value,
            },
        ),
        components::setting_slider_with_step(
            "Fresnel edge",
            format!("{:.2}", tuning.fresnel_strength),
            tuning.fresnel_strength,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::FresnelStrength, value },
        ),
        components::setting_slider_with_step(
            "Hover gain (whole control)",
            format!("{:.0}%", tuning.hover_gain * 100.0),
            tuning.hover_gain,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::HoverGain, value },
        ),
        components::setting_slider_with_step(
            "Press gain (whole control)",
            format!("{:.0}%", tuning.press_gain * 100.0),
            tuning.press_gain,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::PressGain, value },
        ),
        components::setting_slider_with_step(
            "Press tint lift",
            format!("{:.0}%", tuning.press_lift * 100.0),
            tuning.press_lift,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::PressLift, value },
        ),
        components::setting_slider_with_step(
            "Centre light (uniform)",
            format!("{:.3}", tuning.uniform_light),
            tuning.uniform_light,
            0.0..=0.60,
            0.005,
            |value| Message::TuningChanged { parameter: TuningParameter::UniformLight, value },
        ),
        components::setting_slider_with_step(
            "Centre light (lower)",
            format!("{:.3}", tuning.thin_light_gain),
            tuning.thin_light_gain,
            0.0..=0.60,
            0.005,
            |value| Message::TuningChanged { parameter: TuningParameter::ThinLightGain, value },
        ),
    ]);

    container(
        column![
            text("Physical material tuning")
                .size(font::size::TITLE)
                .font(font::ui_font(iced::font::Weight::Semibold)),
            text("Applies to every traffic-light sample")
                .size(font::size::CAPTION)
                .style(components::secondary_text),
            scrollable(rows).height(Length::Fixed(500.0)),
        ]
        .spacing(10),
    )
    .width(Length::Fixed(440.0))
    .padding(16)
    .style(|theme| container::Style {
        background: Some(Background::Color(UiTheme::from_iced(theme).palette().group_background)),
        border: iced::Border::default().rounded(UiCornerStyle::GROUP.with_radius(16.0).radius()),
        ..container::Style::default()
    })
    .into()
}

fn stage_background(scheme: UiColorScheme) -> AppElement<'static> {
    // Use the same neutral substrate as the rest of the macOS window instead
    // of the old four-colour calibration board. The controls remain easy to
    // inspect, but their transmission is judged against a titlebar-like
    // surface rather than an artificial saturated backdrop.
    color_panel(UiTheme::new(scheme).palette().window_background)
}

fn color_panel(color: Color) -> AppElement<'static> {
    container(space())
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(color)),
            ..container::Style::default()
        })
        .into()
}

fn transparent_surface(_theme: &Theme) -> container::Style {
    container::Style::default()
}

fn sample_label(label: &'static str) -> AppElement<'static> {
    container(text(label).size(font::size::BODY).font(font::ui_font(iced::font::Weight::Semibold)))
        .padding([4, 8])
        .style(|theme| container::Style {
            background: Some(Background::Color(
                UiTheme::from_iced(theme).palette().group_background,
            )),
            border: iced::Border::default().rounded(UiCornerStyle::MENU.radius()),
            ..container::Style::default()
        })
        .into()
}

fn positioned(content: AppElement<'_>, x: f32, y: f32) -> AppElement<'_> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding { top: y, right: 0.0, bottom: 0.0, left: x })
        .into()
}

fn window_drag_region() -> AppElement<'static> {
    let content = container(space())
        .width(Length::Fill)
        .height(Length::Fixed(iced_backend::FUSED_TOP_BAR_HEIGHT));
    WindowDragArea::new(content, Message::BeginWindowDrag).into_element()
}

fn main() -> iced::Result {
    let fonts = font::ui_fonts();
    let window_settings = demo_window_policy().apply(iced::window::Settings::default());

    let mut app = iced::application::<State, Message, Theme, Renderer>(boot, update, view)
        .title("Liquid Glass Window Controls")
        .theme(app_theme)
        .subscription(subscription)
        .window(window_settings)
        .window_size(iced::Size::new(1120.0, 720.0));
    for bytes in &fonts.bytes {
        app = app.font(bytes.clone());
    }
    if let Some(ui_font) = fonts.font() {
        app = app.default_font(ui_font);
    }
    app.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sample_owns_a_distinct_state_and_glass_id_group() {
        let mut seen = std::collections::HashSet::new();
        for (index, sample) in SAMPLES.iter().enumerate() {
            assert!(seen.insert(sample.group.index()), "duplicate group index");
            assert_eq!(sample.group.index(), index, "sample order must match ControlGroup");
            assert_eq!(sample.ids.len(), 3);
        }
    }

    #[test]
    fn only_the_measured_sample_executes_window_commands() {
        let executing: Vec<_> = SAMPLES.iter().filter(|sample| sample.executes).collect();
        assert_eq!(executing.len(), 1);
        assert_eq!(executing[0].group, ControlGroup::Native);
    }

    #[test]
    fn measured_and_large_samples_use_distinct_sizes() {
        assert_eq!(WINDOW_CONTROL_NATIVE_SIZE, 14.0);
        assert_eq!(WINDOW_CONTROL_LARGE_SIZE, 64.0);
        assert!(WINDOW_CONTROL_LARGE_SIZE > WINDOW_CONTROL_NATIVE_SIZE);
    }

    #[test]
    fn describe_reports_committed_and_aborted_releases() {
        assert!(
            describe(TrafficLightsEvent::PressEnd { index: 0, committed: true })
                .contains("released")
        );
        assert!(
            describe(TrafficLightsEvent::PressEnd { index: 0, committed: false })
                .contains("no action")
        );
        assert!(describe(TrafficLightsEvent::PressCancel(1)).contains("cancelled"));
    }

    #[test]
    fn interaction_state_is_per_group() {
        let mut state = State::default();
        state.lights[ControlGroup::Active.index()].on_press_start(1);
        assert!(state.lights[ControlGroup::Active.index()].armed.is_some());
        assert!(state.lights[ControlGroup::Native.index()].armed.is_none());
    }

    #[test]
    fn sample_style_reads_its_own_state_machine() {
        let mut state = State::default();
        state.lights[ControlGroup::Disabled.index()].on_press_start(2);
        for _ in 0..30 {
            state.lights[ControlGroup::Disabled.index()].advance(1.0 / 60.0);
        }
        let style = sample_style(SAMPLES[ControlGroup::Disabled.index()], &state);
        assert!(style.scales[2] > 1.0 || style.press[2] > 0.0);
        let untouched = sample_style(SAMPLES[ControlGroup::Native.index()], &state);
        assert_eq!(untouched.press, [0.0; 3]);
    }

    #[test]
    fn inactive_sample_ignores_window_focus() {
        let mut state = State::default();
        state.focused = false;
        let inactive = sample_style(SAMPLES[ControlGroup::Inactive.index()], &state);
        assert!(!inactive.is_active, "the inactive sample is always unfocused");
        let native = sample_style(SAMPLES[ControlGroup::Native.index()], &state);
        assert!(!native.is_active, "the measured sample follows the real window focus");
    }
}
