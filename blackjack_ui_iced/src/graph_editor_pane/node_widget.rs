use blackjack_engine::graph::BjkNodeId;
use blackjack_engine::prelude::Itertools;
use iced_native::Renderer;

use crate::prelude::iced_prelude::*;
use crate::prelude::*;

use super::port_widget::PortWidget;
use super::GraphPaneMessage;
use std::iter::once;

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
    pub node_id: BjkNodeId,
    pub titlebar_left: BjkUiElement<'a>,
    pub titlebar_right: BjkUiElement<'a>,
    pub rows: Vec<NodeRow<'a>>,
    pub bottom_ui: BjkUiElement<'a>,
    pub v_separation: f32,
    pub h_separation: f32,
    pub extra_v_separation: f32,
}

#[derive(Debug)]
struct NodeWidgetState {
    /// The mouse position the previous time we checked for movement. Useful to
    /// compute mouse deltas.
    prev_mouse_pos: Option<Point>,
    /// Is the node currently being dragged?
    is_dragging: bool,
    /// The offset, in units, between the node's origin and the point where the
    /// cursor grabbed it.
    drag_offset: Vector,
}

/// A macro to iterate the children of the node widget, to delegate the various
/// operations on its children.
///
/// It's easier to use a macro than a function here, because a function call
/// boundary imposes additional restrictions on the borrow checker.
macro_rules! iter_stuff {
    ($self:tt) => {
        once(&$self.titlebar_left)
            .chain(once(&$self.titlebar_right))
            .chain(
                $self
                    .rows
                    .iter()
                    .flat_map(|r| [&r.contents, &r.input_port, &r.output_port]),
            )
            .chain(once(&$self.bottom_ui))
    };
    ($self:tt, $layout:ident, $state:ident) => {
        iter_stuff!($self)
            .zip($state.children.iter())
            .zip($layout.children())
    };

    (mut $self:tt) => {
        once(&mut $self.titlebar_left)
            .chain(once(&mut $self.titlebar_right))
            .chain(
                $self
                    .rows
                    .iter_mut()
                    .flat_map(|r| [&mut r.contents, &mut r.input_port, &mut r.output_port]),
            )
            .chain(once(&mut $self.bottom_ui))
    };
    (mut $self:tt, $layout:ident, $state:ident) => {
        iter_stuff!(mut $self)
            .zip($state.children.iter_mut())
            .zip($layout.children())
    };
}

impl<'a> Widget<BjkUiMessage, BjkUiRenderer> for NodeWidget<'a> {
    fn tag(&self) -> WidgetTag {
        WidgetTag::of::<NodeWidgetState>()
    }

    fn state(&self) -> WidgetState {
        WidgetState::new(NodeWidgetState {
            prev_mouse_pos: None,
            is_dragging: false,
            drag_offset: Vector::new(0.0, 0.0),
        })
    }

    fn width(&self) -> Length {
        Length::Shrink
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(&self, renderer: &BjkUiRenderer, limits: &Limits) -> LayoutNode {
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

        let layout_widget = |w: &BjkUiElement, c: &mut Cursor| -> LayoutNode {
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
        cursor.total_size.width = cursor
            .total_size
            .width
            .max(title_left_layout.size().width + title_right_layout.size().width);
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
            let left = LayoutNode::new(Size::new(size, size))
                .translate(Vector::new(-size * 0.5, y_midpoint - size * 0.5));
            let right = LayoutNode::new(Size::new(size, size)).translate(Vector::new(
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

    fn children(&self) -> Vec<WidgetTree> {
        let mut ch = vec![];
        ch.push(WidgetTree::new(&self.titlebar_left));
        ch.push(WidgetTree::new(&self.titlebar_right));
        for row in &self.rows {
            ch.push(WidgetTree::new(&row.contents));
            ch.push(WidgetTree::new(&row.input_port));
            ch.push(WidgetTree::new(&row.output_port));
        }
        ch.push(WidgetTree::new(&self.bottom_ui));
        ch
    }

    fn mouse_interaction(
        &self,
        state: &WidgetTree,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
        renderer: &BjkUiRenderer,
    ) -> MouseInteraction {
        for ((ch, state), layout) in iter_stuff!(self, layout, state) {
            let interaction = ch.as_widget().mouse_interaction(
                state,
                layout,
                cursor_position,
                viewport,
                renderer,
            );
            if interaction != MouseInteraction::Idle {
                return interaction;
            }
        }

        MouseInteraction::Idle
    }

    fn on_event(
        &mut self,
        state: &mut WidgetTree,
        event: iced::Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &BjkUiRenderer,
        clipboard: &mut dyn iced_native::Clipboard,
        shell: &mut iced_native::Shell<'_, BjkUiMessage>,
    ) -> EventStatus {
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
            if status == EventStatus::Captured {
                return status;
            }
        }

        let mut status = EventStatus::Ignored;
        let state = state.state.downcast_mut::<NodeWidgetState>();
        match state.is_dragging {
            false => {
                let titlebar_rect = self.titlebar_rect(&layout);
                if titlebar_rect.contains(cursor_position) {
                    if let iced::Event::Mouse(iced::mouse::Event::ButtonPressed(b)) = event {
                        if b == MouseButton::Left {
                            state.is_dragging = true;
                            state.drag_offset =
                                cursor_position.to_vector() - titlebar_rect.top_left().to_vector();
                            state.prev_mouse_pos = Some(cursor_position);
                            status = EventStatus::Captured;
                        }
                    }
                }
            }
            true => {
                if let iced::Event::Mouse(m) = event {
                    match m {
                        iced::mouse::Event::CursorMoved { .. } => {
                            let delta = cursor_position - state.prev_mouse_pos.unwrap();
                            state.prev_mouse_pos = Some(cursor_position);
                            shell.publish(BjkUiMessage::GraphPane(GraphPaneMessage::NodeMoved {
                                node_id: self.node_id,
                                delta,
                            }))
                        }
                        iced::mouse::Event::ButtonReleased(b) => {
                            if b == MouseButton::Left {
                                state.is_dragging = false;
                                status = EventStatus::Captured;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        status
    }

    fn draw(
        &self,
        state: &WidgetTree,
        renderer: &mut BjkUiRenderer,
        theme: &BjkUiTheme,
        style: &iced_native::renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) {
        let border_radius = 5.0;

        let mut node_rect = layout.bounds();
        node_rect.height += self.extra_v_separation;

        renderer.fill_quad(
            Quad {
                bounds: node_rect,
                border_radius,
                border_width: 0.0,
                border_color: Color::WHITE,
            },
            Color::from_rgb8(63, 63, 63),
        );

        let mut title_rect = self.titlebar_rect(&layout);
        title_rect.height += border_radius;

        renderer.fill_quad(
            Quad {
                bounds: title_rect,
                border_radius,
                border_width: 0.0,
                border_color: Color::WHITE,
            },
            Color::from_rgb8(50, 50, 50),
        );

        // HACK We draw an extra quad to remove the bottom border radius of the
        // title. This is a hack to work around the lack of a per-corner radius.
        let mut title_patch_rect = title_rect;
        title_patch_rect.height = border_radius;
        title_patch_rect.y += title_rect.height - border_radius;
        renderer.fill_quad(
            Quad {
                bounds: title_patch_rect,
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::WHITE,
            },
            Color::from_rgb8(63, 63, 63),
        );

        for ((ch, state), layout) in iter_stuff!(self, layout, state) {
            ch.as_widget().draw(
                state,
                renderer,
                theme,
                style,
                layout,
                cursor_position,
                viewport,
            )
        }
    }

    fn diff(&self, tree: &mut WidgetTree) {
        // We need to allocate here because `diff_children` requires a slice of
        // borrows, not an iterator.
        let children = iter_stuff!(self).collect_vec();
        tree.diff_children(&children);
    }
}

impl NodeWidget<'_> {
    /// Returns the bounding box of the titlebar, given the `layout` tree.
    fn titlebar_rect(&self, layout: &Layout) -> Rectangle {
        let node = layout.bounds();
        let tb_left = layout.children().next().unwrap().bounds();
        let tb_right = layout.children().nth(1).unwrap().bounds();

        Rectangle {
            x: tb_left.x - self.h_separation,
            y: tb_left.y,
            width: node.width,
            height: tb_left.height.max(tb_right.height) + self.extra_v_separation,
        }
    }
}
