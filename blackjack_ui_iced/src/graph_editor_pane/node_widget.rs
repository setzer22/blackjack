use blackjack_engine::graph::BjkNodeId;
use blackjack_engine::prelude::Itertools;
use iced_native::Renderer;

use crate::prelude::iced_prelude::*;
use crate::prelude::*;

use super::GraphPaneMessage;
use std::iter::once;

struct NodePort {
    pub color: Color,
}

pub struct NodeRow<'a> {
    input_port: Option<NodePort>,
    pub contents: BjkUiElement<'a>,
    output_port: Option<NodePort>,
}

impl<'a> NodeRow<'a> {
    pub const PORT_RADIUS: f32 = 5.0;

    pub fn input(contents: impl Into<BjkUiElement<'a>>, color: Color) -> Self {
        Self {
            input_port: Some(NodePort { color }),
            contents: contents.into(),
            output_port: None,
        }
    }

    pub fn output(contents: impl Into<BjkUiElement<'a>>, color: Color) -> Self {
        Self {
            input_port: None,
            contents: contents.into(),
            output_port: Some(NodePort { color }),
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NodeEventStatus {
    Ignored,
    BeingDragged,
    CapturedByWidget,
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
            .chain($self.rows.iter().map(|r| &r.contents))
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
            .chain($self.rows.iter_mut().map(|r| &mut r.contents))
            .chain(once(&mut $self.bottom_ui))
    };
    (mut $self:tt, $layout:ident, $state:ident) => {
        iter_stuff!(mut $self)
            .zip($state.children.iter_mut())
            .zip($layout.children())
    };
}

impl NodeWidget<'_> {
    pub fn tag(&self) -> WidgetTag {
        WidgetTag::of::<NodeWidgetState>()
    }

    // TODO: If node ends up having no state, remove.
    pub fn state(&self) -> WidgetState {
        WidgetState::new(NodeWidgetState {
            prev_mouse_pos: None,
            is_dragging: false,
            drag_offset: Vector::new(0.0, 0.0),
        })
    }

    pub fn children(&self) -> Vec<WidgetTree> {
        let mut children = vec![];
        for ch in iter_stuff!(self) {
            children.push(WidgetTree::new(ch));
        }
        children
    }

    pub fn diff(&self, tree: &mut WidgetTree) {
        let child_refs = iter_stuff!(self).collect_vec();
        tree.diff_children(&child_refs);
    }

    pub fn layout(&self, renderer: &BjkUiRenderer, limits: &Limits) -> LayoutNode {
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
        let mut row_contents = vec![];
        for row in &self.rows {
            let row_layout = layout_widget(&row.contents, &mut cursor);
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

        let mut children = vec![];
        children.push(title_left_layout);
        children.push(title_right_layout);
        for row in row_contents {
            children.push(row);
        }
        children.push(bottom_ui_layout);

        iced_native::layout::Node::with_children(cursor.total_size, children)
    }

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

    /// Returns the visual information of the left and right port (both
    /// optional) for the `row-idx`-th row
    #[allow(clippy::type_complexity)]
    fn port_visuals(
        &self,
        layout: &Layout,
        row_idx: usize,
        row: &NodeRow,
    ) -> (Option<(Point, Color)>, Option<(Point, Color)>) {
        let row_bounds = layout.children().nth(row_idx + 2).unwrap().bounds();
        let node_bounds = layout.bounds();
        let left = Point::new(node_bounds.x, row_bounds.center_y());
        let right = Point::new(node_bounds.x + node_bounds.width, row_bounds.center_y());
        (
            row.input_port.as_ref().map(|i| (left, i.color)),
            row.output_port.as_ref().map(|o| (right, o.color)),
        )
    }

    pub fn draw(
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

        for (i, row) in self.rows.iter().enumerate() {
            let mut draw_circle = |center: Point, radius, color| {
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle {
                            x: center.x - radius,
                            y: center.y - radius,
                            width: radius * 2.0,
                            height: radius * 2.0,
                        },
                        border_radius: radius,
                        border_width: 0.0,
                        border_color: Color::TRANSPARENT,
                    },
                    Background::Color(color),
                );
            };
            let hover_color = |pos, color: Color| {
                if cursor_position.distance(pos) < NodeRow::PORT_RADIUS {
                    color.add(0.6)
                } else {
                    color
                }
            };

            let (left, right) = self.port_visuals(&layout, i, row);
            if let Some((left, color)) = left {
                draw_circle(left, NodeRow::PORT_RADIUS, hover_color(left, color));
            }
            if let Some((right, color)) = right {
                draw_circle(right, NodeRow::PORT_RADIUS, hover_color(right, color));
            }
        }
    }

    pub fn mouse_interaction(
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

    pub fn on_event(
        &mut self,
        state: &mut WidgetTree,
        event: iced::Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &BjkUiRenderer,
        clipboard: &mut dyn iced_native::Clipboard,
        shell: &mut iced_native::Shell<'_, BjkUiMessage>,
    ) -> NodeEventStatus {
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
                return NodeEventStatus::CapturedByWidget;
            }
        }

        let mut status = NodeEventStatus::Ignored;
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
                            status = NodeEventStatus::BeingDragged;
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
                            }));
                            status = NodeEventStatus::BeingDragged;
                        }
                        iced::mouse::Event::ButtonReleased(b) => {
                            if b == MouseButton::Left {
                                state.is_dragging = false;
                                status = NodeEventStatus::BeingDragged;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        status
    }
}
