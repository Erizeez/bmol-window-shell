//! Application-level window scaffold and declarative chrome assembly.
//!
//! Provides [`WindowScaffold`], an ergonomic builder that combines titlebar
//! layout, Apple traffic lights, loyal drag regions, non-client window rims,
//! and 8-direction interactive resizing into a clean, declarative call.

use iced::advanced::{self, svg as advanced_svg};
use iced::widget::{column, container, row, space, text};
use iced::{Alignment, Color, Element, Length, Padding, window};

use super::controller::WindowShellController;
use super::traffic_lights::TrafficLightsEvent;
use crate::platform::window_metrics;

/// Declarative builder for standard frameless macOS application windows.
///
/// Encapsulates the complete outer non-client shell:
/// - 8-direction border resize handles (`wrap_border_resizer`)
/// - Authentic non-client rim with 0.5/1.0 pt borders (`wrap_window_rim`)
/// - Titlebar with traffic lights, draggable area, and double-click zoom (`loyal_drag_bar`)
/// - Safe client area layout
pub struct WindowScaffold<'a, Message, Theme, Renderer> {
    controller: &'a WindowShellController,
    body: Element<'a, Message, Theme, Renderer>,
    title: Option<&'a str>,
    header_element: Option<Element<'a, Message, Theme, Renderer>>,
    header_trailing: Option<Element<'a, Message, Theme, Renderer>>,
    header_height: Option<f32>,
    footer: Option<Element<'a, Message, Theme, Renderer>>,
    corner_radius: f32,
    on_drag: Option<Message>,
    on_resize: Option<Box<dyn Fn(window::Direction) -> Message + 'a>>,
    on_traffic_lights: Option<Box<dyn Fn(TrafficLightsEvent) -> Message + 'a>>,
    double_click_zoom: bool,
}

impl<Message, Theme, Renderer> std::fmt::Debug
    for WindowScaffold<'_, Message, Theme, Renderer>
{
    /// The scaffold holds boxed closures for the drag, resize and traffic-light
    /// callbacks, so it cannot derive `Debug`; report the data it does have.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WindowScaffold")
            .field("title", &self.title)
            .field("corner_radius", &self.corner_radius)
            .field("double_click_zoom", &self.double_click_zoom)
            .finish_non_exhaustive()
    }
}

impl<'a, Message: 'a + Clone, Theme, Renderer> WindowScaffold<'a, Message, Theme, Renderer>
where
    Theme: 'a + iced::widget::container::Catalog + iced::widget::svg::Catalog + iced::widget::text::Catalog,
    <Theme as iced::widget::container::Catalog>::Class<'a>:
        From<iced::widget::container::StyleFn<'a, Theme>>,
    <Theme as iced::widget::svg::Catalog>::Class<'a>:
        From<iced::widget::svg::StyleFn<'a, Theme>>,
    <Theme as iced::widget::text::Catalog>::Class<'a>:
        From<iced::widget::text::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + advanced_svg::Renderer + advanced::text::Renderer + 'a,
{
    /// Creates a new scaffold wrapping the given client area content.
    pub fn new(
        controller: &'a WindowShellController,
        body: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self {
            controller,
            body: body.into(),
            title: None,
            header_element: None,
            header_trailing: None,
            header_height: None,
            footer: None,
            corner_radius: window_metrics::DEFAULT_CORNER_RADIUS,
            on_drag: None,
            on_resize: None,
            on_traffic_lights: None,
            double_click_zoom: true,
        }
    }

    /// Sets the text title shown in the window header bar.
    #[must_use]
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Sets a completely custom header element.
    ///
    /// If supplied, this replaces the automatic titlebar while still
    /// being wrapped in the draggable region and window rim.
    #[must_use]
    pub fn header(mut self, header: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        self.header_element = Some(header.into());
        self
    }

    /// Appends trailing actions / controls to the right side of the automatic header.
    #[must_use]
    pub fn header_trailing(
        mut self,
        trailing: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        self.header_trailing = Some(trailing.into());
        self
    }

    /// Overrides the header height (defaults to the controller metrics' header height).
    #[must_use]
    pub fn header_height(mut self, height: f32) -> Self {
        self.header_height = Some(height);
        self
    }

    /// Sets an optional bottom status bar or footer.
    #[must_use]
    pub fn footer(mut self, footer: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Overrides the window corner radius.
    #[must_use]
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Enables window dragging on the header bar using the given message.
    #[must_use]
    pub fn on_drag(mut self, message: Message) -> Self {
        self.on_drag = Some(message);
        self
    }

    /// Enables 8-direction interactive border resizing using the given callback.
    #[must_use]
    pub fn on_resize(mut self, f: impl Fn(window::Direction) -> Message + 'a) -> Self {
        self.on_resize = Some(Box::new(f));
        self
    }

    /// Wire in the traffic lights event dispatcher.
    #[must_use]
    pub fn on_traffic_lights(
        mut self,
        f: impl Fn(TrafficLightsEvent) -> Message + 'a,
    ) -> Self {
        self.on_traffic_lights = Some(Box::new(f));
        self
    }

    /// Sets whether double-clicking the titlebar triggers window zoom/maximize.
    #[must_use]
    pub fn double_click_zoom(mut self, enabled: bool) -> Self {
        self.double_click_zoom = enabled;
        self
    }

    /// Assembles the complete window into a single Iced root [`Element`].
    pub fn build(self) -> Element<'a, Message, Theme, Renderer> {
        let h_height = self
            .header_height
            .unwrap_or(self.controller.metrics.header_rect.height);

        let zoom_action = if self.double_click_zoom {
            self.on_traffic_lights.as_ref().map(|f| {
                f(TrafficLightsEvent::PressEnd { index: 2, committed: true })
            })
        } else {
            None
        };

        // 1. Build or use header
        let header_view: Option<Element<'a, Message, Theme, Renderer>> =
            if let Some(custom) = self.header_element {
                Some(custom)
            } else if self.title.is_some()
                || self.on_traffic_lights.is_some()
                || self.header_trailing.is_some()
            {
                let mut header_row = row![].align_y(Alignment::Center).spacing(8.0);

                if let Some(on_tl) = self.on_traffic_lights {
                    let tl = self.controller.traffic_lights_view(on_tl);
                    header_row = header_row.push(tl);
                }

                if let Some(title_text) = self.title {
                    let title_color = if self.controller.is_dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.90)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.85)
                    };
                    let title_widget = text(title_text)
                        .size(13)
                        .color(title_color);
                    header_row = header_row.push(title_widget);
                }

                header_row = header_row.push(space().width(Length::Fill));

                if let Some(trailing) = self.header_trailing {
                    header_row = header_row.push(trailing);
                }

                let container_widget = container(header_row)
                    .width(Length::Fill)
                    .height(Length::Fixed(h_height))
                    .padding(Padding {
                        top: 0.0,
                        right: 12.0,
                        bottom: 0.0,
                        left: 12.0,
                    })
                    .align_y(Alignment::Center);

                Some(container_widget.into())
            } else {
                None
            };

        // 2. Wrap header in loyal drag bar if on_drag is specified
        let draggable_header = header_view.map(|h| {
            if let Some(drag_msg) = self.on_drag {
                self.controller
                    .loyal_drag_bar(h_height, h, drag_msg, zoom_action)
            } else {
                h
            }
        });

        // 3. Assemble content column: header + body + optional footer
        let mut content = column![].width(Length::Fill).height(Length::Fill);

        if let Some(h) = draggable_header {
            content = content.push(h);
        }

        content = content.push(self.body);

        if let Some(f) = self.footer {
            content = content.push(f);
        }

        // 4. Wrap with non-client rim and interactive resizer
        if let Some(on_resize) = self.on_resize {
            self.controller.wrap_window_with_resizer(
                content,
                self.corner_radius,
                move |d| on_resize(d),
            )
        } else {
            self.controller.wrap_window(content, self.corner_radius)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::WindowChromeConfig;
    use iced::widget::space;

    #[test]
    fn test_window_scaffold_builder() {
        let config = WindowChromeConfig::separate(32.0);
        let controller = WindowShellController::new(config, false);

        let _element: Element<'_, (), iced::Theme, iced::Renderer> = controller
            .scaffold(space())
            .title("Test Window")
            .on_drag(())
            .on_resize(|_| ())
            .on_traffic_lights(|_| ())
            .build();
    }
}

