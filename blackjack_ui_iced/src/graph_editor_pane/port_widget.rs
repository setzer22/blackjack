use iced_native::Renderer;

use crate::prelude::{iced_prelude::*, *};

pub struct PortWidget {
    pub color: Color,
    pub size: f32,
}

impl iced_native::Widget<crate::BjkUiMessage, BjkUiRenderer> for PortWidget {
    fn width(&self) -> Length {
        Length::Shrink
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(&self, _renderer: &BjkUiRenderer, _limits: &Limits) -> LayoutNode {
        LayoutNode::new(Size::new(self.size, self.size))
    }

    // WIP: Finally understood how the diffing algorithm works. It is not
    // necessary to implement it for nodes without children like PortWidget, but
    // I have to do it for the NodeWidget.
    //
    // Diffing is what would add or remove the branches in the widget tree for
    // newly created or removed elements (think, list items in the classic todo
    // app). It is also a more general mechanism than that, it can be used to
    // recoinciliate the widget tree (a.k.a. the inner widget state, what makes
    // widgets *stateless*) when there are changes in the tree.

    fn draw(
        &self,
        _state: &WidgetTree,
        renderer: &mut BjkUiRenderer,
        _theme: &BjkUiTheme,
        _style: &RendererStyle,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
    ) {
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
        )
    }

    /*
    fn on_event(
        &mut self,
        _state: &mut iced_native::widget::Tree,
        _event: iced::Event,
        layout: Layout<'_>,
        cursor_position: Point,
        _renderer: &BjkUiRenderer,
        _clipboard: &mut dyn iced_native::Clipboard,
        _shell: &mut iced_native::Shell<'_, crate::BjkUiMessage>,
    ) -> iced::event::Status {
        let bounds = layout.bounds();
        let in_bounds = bounds.contains(cursor_position);
        match event {
            mouse::Ev
        }
    }*/
}
