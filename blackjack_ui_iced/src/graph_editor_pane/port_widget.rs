use iced_native::Renderer;

use crate::prelude::{iced_prelude::*, *};

pub struct PortWidget {
    pub color: Color,
    pub size: f32,
}

pub struct PortWidgetState {
    pub is_dragging: bool,
}

impl iced_native::Widget<crate::BjkUiMessage, BjkUiRenderer> for PortWidget {
    fn tag(&self) -> WidgetTag {
        WidgetTag::of::<PortWidgetState>()
    }

    fn state(&self) -> WidgetState {
        WidgetState::new(PortWidgetState { is_dragging: false })
    }

    fn width(&self) -> Length {
        Length::Shrink
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(&self, _renderer: &BjkUiRenderer, _limits: &Limits) -> LayoutNode {
        LayoutNode::new(Size::new(self.size, self.size))
    }

    fn draw(
        &self,
        state: &WidgetTree,
        renderer: &mut BjkUiRenderer,
        _theme: &BjkUiTheme,
        _style: &RendererStyle,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
    ) {
        let state = state.state.downcast_ref::<PortWidgetState>();
        let color = if layout.bounds().contains(cursor_position) {
            self.color.add(0.6)
        } else {
            self.color
        };

        renderer.fill_quad(
            Quad {
                bounds: layout.bounds(),
                border_radius: self.size / 2.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
            Background::Color(color),
        );

        if state.is_dragging {
            use iced::widget::canvas::{Frame, Path, Stroke};
            let mut frame = Frame::new(Size::new(1000.0, 1000.0));
            let start = layout.bounds().center();
            let end = cursor_position;
            frame.stroke(
                &Path::line(start, end),
                Stroke::default()
                    .with_width(5.0)
                    .with_color(Color::from_rgb8(77, 84, 92)),
            );
            let primitive = frame.into_geometry().into_primitive();
            renderer.draw_primitive(primitive);

        }
    }

    fn on_event(
        &mut self,
        state: &mut iced_native::widget::Tree,
        event: iced::Event,
        layout: Layout<'_>,
        cursor_position: Point,
        _renderer: &BjkUiRenderer,
        _clipboard: &mut dyn iced_native::Clipboard,
        _shell: &mut iced_native::Shell<'_, crate::BjkUiMessage>,
    ) -> iced::event::Status {
        let state = state.state.downcast_mut::<PortWidgetState>();
        let bounds = layout.bounds();
        let in_bounds = bounds.contains(cursor_position);
        let mut captured = false;

        match state.is_dragging {
            false => {
                if let iced::Event::Mouse(iced::mouse::Event::ButtonPressed(b)) = event {
                    if in_bounds && b == MouseButton::Left {
                        state.is_dragging = true;
                        captured = true;
                    }
                }
            }
            true => {
                if let iced::Event::Mouse(iced::mouse::Event::ButtonReleased(b)) = event {
                    if b == MouseButton::Left {
                        state.is_dragging = false;
                    }
                }
            }
        }

        // TODO: Should this always be captured, if there's an ongoing drag
        // event? Don't think it matters much.
        if captured {
            EventStatus::Captured
        } else {
            EventStatus::Ignored
        }
    }
}
