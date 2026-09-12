//! The state-machine bridge between the traffic-light widget and the window.
//!
//! The widget itself — the Apple vector glyphs, the hit testing, the overlay
//! routing that keeps the glyphs above the glass — lives in the shared widget
//! layer, [`liquid_glass_ui::traffic_light`]. What stays here is the part that
//! knows about *windows*:
//!
//! - [`native_group_style`], which reads one frame of the interaction state
//!   machine and produces the widget's presentation snapshot;
//! - [`view_traffic_lights_all_inclusive`], the pre-fabricated native control
//!   group;
//! - [`MeasuredTrafficLights`], which publishes the glyph row's measured origin
//!   into the shared interaction frame so the GPU spheres and the Iced glyphs
//!   share one coordinate system instead of two that can drift apart.

use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{self, layout, renderer, svg as advanced_svg, Clipboard, Layout, Shell};
use iced::mouse;
use iced::widget::container;
use iced::{Element, Event, Length, Rectangle, Size};

use liquid_glass_ui::traffic_light::{
    ControlGroupStyle, DIAMETER, SPACING, TrafficLightsEvent, control_hover_slop,
    is_document_edited, window_control_group,
};

use crate::interaction::set_window_control_origin;
use crate::state::TrafficLightsState;

/// The widget's presentation snapshot for the standard measured control group.
///
/// This is the one place that reads animation values out of
/// [`TrafficLightsState`] and hands them to the widget, so the rendering code
/// never reaches into the state machine itself.
#[must_use]
pub fn native_group_style(
    state: &TrafficLightsState,
    is_focused: bool,
    is_dark: bool,
) -> ControlGroupStyle {
    let hover = state.hover_progress;
    ControlGroupStyle {
        size: DIAMETER,
        gap: SPACING,
        is_dark,
        is_active: is_focused || hover > 0.05,
        close_disabled: false,
        close_dot: is_document_edited(),
        expand_behavior: state.expand_behavior,
        hover,
        press: [state.press_tint(0), state.press_tint(1), state.press_tint(2)],
        scales: [state.press_scale(0), state.press_scale(1), state.press_scale(2)],
    }
}

/// Builds the pre-fabricated native traffic lights widget bound to a state
/// machine instance.
///
/// The returned element is already routed through the compositor's overlay
/// layer by the widget layer, and it publishes its measured origin so the GPU
/// spheres land on the same pixels as the glyphs.
pub fn view_traffic_lights_all_inclusive<'a, Message: Clone + 'a, Theme, Renderer>(
    state: &'a TrafficLightsState,
    is_focused: bool,
    is_dark: bool,
    on_event: impl Fn(TrafficLightsEvent) -> Message + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: 'a + container::Catalog + iced::widget::svg::Catalog,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    <Theme as iced::widget::svg::Catalog>::Class<'a>: From<iced::widget::svg::StyleFn<'a, Theme>>,
    Renderer:
        advanced::Renderer + advanced_svg::Renderer + liquid_glass_ui::GlassForegroundRenderer + 'a,
{
    let group: Element<'a, Message, Theme, Renderer> = window_control_group(
        [liquid_glass_scene::GlassId(0); 3],
        native_group_style(state, is_focused, is_dark),
        move |_id: liquid_glass_scene::GlassId, event| on_event(event),
    );
    MeasuredTrafficLights::new(group).into()
}

/// Transparent wrapper that publishes the measured absolute origin of the
/// traffic-light glyph row.
///
/// The Liquid Glass compositor reads the same origin for the sphere nodes, so
/// the glyphs and the GPU glass are driven by one real layout measurement
/// instead of two independently-computed coordinate systems.
struct MeasuredTrafficLights<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
}

impl<'a, Message, Theme, Renderer> MeasuredTrafficLights<'a, Message, Theme, Renderer> {
    fn new(content: Element<'a, Message, Theme, Renderer>) -> Self {
        Self { content }
    }
}

impl<Message, Theme, Renderer> std::fmt::Debug
    for MeasuredTrafficLights<'_, Message, Theme, Renderer>
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("MeasuredTrafficLights").finish_non_exhaustive()
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for MeasuredTrafficLights<'_, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let slop = control_hover_slop(DIAMETER);
        // Snap the measured origin to the device-pixel grid (0.5 logical px on a
        // 2x display) so the traffic-light margin stays a clean, reproducible
        // value instead of a fractional product of Iced's centering arithmetic.
        let snap = |value: f32| (value * 2.0).round() / 2.0;
        set_window_control_origin(snap(bounds.x + slop), snap(bounds.y + slop));
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(self.content.as_widget())]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.content.as_widget()]);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn advanced::widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }
}

impl<'a, Message: Clone + 'a, Theme: 'a, Renderer: advanced::Renderer + 'a>
    From<MeasuredTrafficLights<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
{
    fn from(wrapper: MeasuredTrafficLights<'a, Message, Theme, Renderer>) -> Self {
        Element::new(wrapper)
    }
}
