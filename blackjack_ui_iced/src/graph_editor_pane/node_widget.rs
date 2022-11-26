use std::rc::Rc;

use blackjack_engine::prelude::Itertools;
use iced::{
    mouse::Interaction,
    widget::{Row, Space},
    Background, Color, Element, Length, Rectangle, Size, Vector,
};
use iced_native::{
    layout::{self, flex::Axis, Limits, Node},
    renderer,
    widget::Tree,
    Layout, Renderer, Widget,
};

use crate::BlackjackUiMessage;

pub struct NodeWidget<'a> {
    pub titlebar_left: Element<'a, BlackjackUiMessage>,
    pub titlebar_right: Element<'a, BlackjackUiMessage>,
    pub rows: Vec<Element<'a, BlackjackUiMessage>>,
    pub bottom_ui: Element<'a, BlackjackUiMessage>,
    pub v_separation: f32,
    pub h_separation: f32,
    pub extra_v_separation: f32,
}

impl<'a> NodeWidget<'a> {
    fn iter_stuff(
        &'a self,
        state: &'a Tree,
        layout: Layout<'a>,
    ) -> impl Iterator<
        Item = (
            &Element<'a, BlackjackUiMessage, iced::Renderer>,
            &Tree,
            Layout,
        ),
    > {
        use std::iter::once;
        once(&self.titlebar_left)
            .chain(once(&self.titlebar_right))
            .chain(self.rows.iter())
            .chain(once(&self.bottom_ui))
            .zip(state.children.iter())
            .zip(layout.children())
            .map(|((x, y), z)| (x, y, z))
    }
}

impl<'a> Widget<BlackjackUiMessage, iced::Renderer> for NodeWidget<'a> {
    fn width(&self) -> Length {
        Length::Shrink
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(
        &self,
        renderer: &iced::Renderer,
        limits: &iced_native::layout::Limits,
    ) -> iced_native::layout::Node {
        struct Cursor {
            y_offset: f32,
            limits: Limits,
            total_size: Size,
            children: Vec<Node>,
        }

        let mut cursor = Cursor {
            y_offset: self.v_separation,
            limits: *limits,
            total_size: Size::<f32>::new(0.0, 0.0),
            children: vec![],
        };

        let layout_widget = |w: &Element<_, _>, c: &mut Cursor| {
            let layout = w.as_widget().layout(renderer, &c.limits);
            let size = layout.size();
            c.limits = c
                .limits
                .shrink(Size::new(0.0, size.height + self.v_separation));
            c.total_size.width = c.total_size.width.max(size.width);
            c.total_size.height += size.height + self.v_separation;
            c.children.push(layout.translate(Vector {
                x: self.h_separation,
                y: c.y_offset,
            }));
            c.y_offset += size.height + self.v_separation;
        };

        // Make room for the title, which we will layout at the end, so we can
        // set its max width to the width of the node.
        let title_left_layout = self.titlebar_left.as_widget().layout(renderer, limits);
        let title_right_layout = self.titlebar_right.as_widget().layout(renderer, limits);
        let title_height = title_left_layout
            .size()
            .height
            .max(title_right_layout.size().height)
            + self.extra_v_separation;
        cursor.y_offset += title_height;
        cursor.total_size.height += title_height;

        // Layout rows
        for row in &self.rows {
            layout_widget(row, &mut cursor);
        }

        // Layout bottom UI
        layout_widget(&self.bottom_ui, &mut cursor);

        // Layout titlebar
        let trl_width = title_right_layout.bounds().width;
        let title_left_layout = title_left_layout.translate(Vector {
            x: self.h_separation,
            y: 0.0,
        });
        let title_right_layout = title_right_layout.translate(Vector {
            x: cursor.total_size.width - trl_width + self.h_separation,
            y: 0.0,
        });
        cursor.children.insert(0, title_left_layout);
        cursor.children.insert(1, title_right_layout);

        cursor.total_size.height += self.v_separation;
        cursor.total_size.width += 2.0 * self.h_separation;

        iced_native::layout::Node::with_children(cursor.total_size, cursor.children)
    }

    fn children(&self) -> Vec<iced_native::widget::Tree> {
        let mut ch = vec![];
        ch.push(Tree::new(&self.titlebar_left));
        ch.push(Tree::new(&self.titlebar_right));
        for row in &self.rows {
            ch.push(Tree::new(row));
        }
        ch.push(Tree::new(&self.bottom_ui));
        ch
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: iced_native::Layout<'_>,
        cursor_position: iced::Point,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> iced_native::mouse::Interaction {
        use std::iter::once;
        for ((ch, state), layout) in once(&self.titlebar_left)
            .chain(once(&self.titlebar_right))
            .chain(self.rows.iter())
            .chain(once(&self.bottom_ui))
            .zip(state.children.iter())
            .zip(layout.children())
        {
            let interaction = ch.as_widget().mouse_interaction(
                state,
                layout,
                cursor_position,
                viewport,
                renderer,
            );
            if interaction != Interaction::Idle {
                return interaction;
            }
        }
        iced_native::mouse::Interaction::Idle
    }

    fn on_event(
        &mut self,
        state: &mut iced_native::widget::Tree,
        event: iced::Event,
        layout: iced_native::Layout<'_>,
        cursor_position: iced::Point,
        renderer: &iced::Renderer,
        clipboard: &mut dyn iced_native::Clipboard,
        shell: &mut iced_native::Shell<'_, BlackjackUiMessage>,
    ) -> iced::event::Status {
        use std::iter::once;
        for ((ch, state), layout) in once(&mut self.titlebar_left)
            .chain(once(&mut self.titlebar_right))
            .chain(self.rows.iter_mut())
            .chain(once(&mut self.bottom_ui))
            .zip(state.children.iter_mut())
            .zip(layout.children())
        {
            let status = ch.as_widget_mut().on_event(
                state,
                event.clone(),
                layout,
                cursor_position,
                renderer,
                clipboard,
                shell,
            );
            if status == iced::event::Status::Captured {
                return status;
            }
        }

        // TODO: Handle self mouse events here
        iced::event::Status::Ignored
    }

    fn draw(
        &self,
        state: &iced_native::widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &iced_native::Theme,
        style: &renderer::Style,
        layout: iced_native::Layout<'_>,
        cursor_position: iced::Point,
        viewport: &iced::Rectangle,
    ) {
        let style = renderer::Style {
            text_color: Color::from_rgb8(227, 227, 227),
        };
        let border_radius = 5.0;

        let mut node_rect = layout.bounds();
        node_rect.height += self.extra_v_separation;

        renderer.fill_quad(
            renderer::Quad {
                bounds: node_rect,
                border_radius,
                border_width: 0.0,
                border_color: Color::WHITE,
            },
            Color::from_rgb8(63, 63, 63),
        );

        let titlebar_height = layout
            .children()
            .take(2)
            .map(|x| x.bounds().height)
            .max_by(f32::total_cmp)
            .unwrap();
        let mut title_rect = layout.bounds();
        title_rect.height = titlebar_height + self.extra_v_separation + border_radius;

        renderer.fill_quad(
            renderer::Quad {
                bounds: title_rect,
                border_radius,
                border_width: 0.0,
                border_color: Color::WHITE,
            },
            Color::from_rgb8(50, 50, 50),
        );

        // HACK We draw an extra quad to remove the bottom border radius of the
        // title. This is a hack to work around the lack of a per-corner radius.
        let mut title_patch_rect = layout.bounds();
        title_patch_rect.height = border_radius;
        title_patch_rect.y += titlebar_height + self.extra_v_separation;
        renderer.fill_quad(
            renderer::Quad {
                bounds: title_patch_rect,
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::WHITE,
            },
            Color::from_rgb8(63, 63, 63),
        );

        use std::iter::once;
        for ((ch, state), layout) in once(&self.titlebar_left)
            .chain(once(&self.titlebar_right))
            .chain(self.rows.iter())
            .chain(once(&self.bottom_ui))
            .zip(state.children.iter())
            .zip(layout.children())
        {
            ch.as_widget().draw(
                state,
                renderer,
                theme,
                &style,
                layout,
                cursor_position,
                viewport,
            )
        }
    }
}
