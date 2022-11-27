use iced::{mouse::Interaction, Color, Length, Rectangle, Size, Vector};
use iced_native::{
    layout::{Limits, Node},
    renderer,
    widget::Tree,
    Renderer, Widget,
};

use crate::prelude::*;

use super::port_widget::PortWidget;

pub struct NodeRow<'a> {
    pub input_port: BjkUiElement<'a>,
    pub contents: BjkUiElement<'a>,
    pub output_port: BjkUiElement<'a>,
}

impl<'a> NodeRow<'a> {
    const PORT_SIZE: f32 = 10.0;

    pub fn input(contents: impl Into<BjkUiElement<'a>>, color: Color) -> Self {
        Self {
            input_port: BjkUiElement::new(PortWidget {
                color,
                size: Self::PORT_SIZE,
            }),
            contents: contents.into(),
            output_port: BjkUiElement::new(empty_space()),
        }
    }

    pub fn output(contents: impl Into<BjkUiElement<'a>>, color: Color) -> Self {
        Self {
            input_port: BjkUiElement::new(empty_space()),
            contents: contents.into(),
            output_port: BjkUiElement::new(PortWidget {
                color,
                size: Self::PORT_SIZE,
            }),
        }
    }
}

pub struct NodeWidget<'a> {
    pub titlebar_left: BjkUiElement<'a>,
    pub titlebar_right: BjkUiElement<'a>,
    pub rows: Vec<NodeRow<'a>>,
    pub bottom_ui: BjkUiElement<'a>,
    pub v_separation: f32,
    pub h_separation: f32,
    pub extra_v_separation: f32,
}

macro_rules! iter_stuff {
    ($self:tt, $layout:ident, $state:ident) => {
        once(&$self.titlebar_left)
            .chain(once(&$self.titlebar_right))
            .chain(
                $self
                    .rows
                    .iter()
                    .flat_map(|r| [&r.contents, &r.input_port, &r.output_port]),
            )
            .chain(once(&$self.bottom_ui))
            .zip($state.children.iter())
            .zip($layout.children())
    };

    (mut $self:tt, $layout:ident, $state:ident) => {
        once(&mut $self.titlebar_left)
            .chain(once(&mut $self.titlebar_right))
            .chain(
                $self
                    .rows
                    .iter_mut()
                    .flat_map(|r| [&mut r.contents, &mut r.input_port, &mut r.output_port]),
            )
            .chain(once(&mut $self.bottom_ui))
            .zip($state.children.iter_mut())
            .zip($layout.children())
    };
}

impl<'a> Widget<BjkUiMessage, BjkUiRenderer> for NodeWidget<'a> {
    fn width(&self) -> Length {
        Length::Shrink
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(
        &self,
        renderer: &BjkUiRenderer,
        limits: &iced_native::layout::Limits,
    ) -> iced_native::layout::Node {
        struct Cursor {
            y_offset: f32,
            limits: Limits,
            total_size: Size,
        }

        let mut cursor = Cursor {
            y_offset: self.v_separation,
            limits: *limits,
            total_size: Size::<f32>::new(0.0, 0.0),
        };

        let layout_widget = |w: &BjkUiElement, c: &mut Cursor| -> Node {
            let layout = w.as_widget().layout(renderer, &c.limits);
            let size = layout.size();
            c.limits = c
                .limits
                .shrink(Size::new(0.0, size.height + self.v_separation));
            c.total_size.width = c.total_size.width.max(size.width);
            c.total_size.height += size.height + self.v_separation;
            let layout = layout.translate(Vector {
                x: self.h_separation,
                y: c.y_offset,
            });
            c.y_offset += size.height + self.v_separation;
            layout
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

        // Layout row contents
        let mut row_y_midpoints = vec![];
        let mut row_contents = vec![];
        for row in &self.rows {
            let row_layout = layout_widget(&row.contents, &mut cursor);
            row_y_midpoints.push(row_layout.bounds().center_y());
            row_contents.push(row_layout);
        }

        // Layout bottom UI
        let bottom_ui_layout = layout_widget(&self.bottom_ui, &mut cursor);

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

        cursor.total_size.height += self.v_separation;
        cursor.total_size.width += 2.0 * self.h_separation;

        // Layout ports
        let mut port_layouts = vec![];
        for y_midpoint in row_y_midpoints {
            let size = NodeRow::PORT_SIZE;
            let left = Node::new(Size::new(size, size))
                .translate(Vector::new(-size * 0.5, y_midpoint - size * 0.5));
            let right = Node::new(Size::new(size, size)).translate(Vector::new(
                cursor.total_size.width - size * 0.5,
                y_midpoint - size * 0.5,
            ));
            port_layouts.push((left, right));
        }

        let mut children = vec![];
        children.push(title_left_layout);
        children.push(title_right_layout);
        for (row, (left, right)) in row_contents.into_iter().zip(port_layouts) {
            children.push(row);
            children.push(left);
            children.push(right);
        }
        children.push(bottom_ui_layout);

        iced_native::layout::Node::with_children(cursor.total_size, children)
    }

    fn children(&self) -> Vec<iced_native::widget::Tree> {
        println!("Children");

        let mut ch = vec![];
        ch.push(Tree::new(&self.titlebar_left));
        ch.push(Tree::new(&self.titlebar_right));
        for row in &self.rows {
            ch.push(Tree::new(&row.contents));
            ch.push(Tree::new(&row.input_port));
            ch.push(Tree::new(&row.output_port));
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
        renderer: &BjkUiRenderer,
    ) -> iced_native::mouse::Interaction {
        use std::iter::once;
        for ((ch, state), layout) in iter_stuff!(self, layout, state) {
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
        renderer: &BjkUiRenderer,
        clipboard: &mut dyn iced_native::Clipboard,
        shell: &mut iced_native::Shell<'_, BjkUiMessage>,
    ) -> iced::event::Status {
        use std::iter::once;
        for ((ch, state), layout) in iter_stuff!(mut self, layout, state) {
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
        renderer: &mut BjkUiRenderer,
        theme: &BjkUiTheme,
        _style: &renderer::Style,
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
        for ((ch, state), layout) in iter_stuff!(self, layout, state) {
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
