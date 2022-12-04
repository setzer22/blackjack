use blackjack_engine::prelude::Itertools;
use iced_graphics::Transformation;
use iced_native::Renderer;

use crate::prelude::iced_prelude::*;
use crate::prelude::*;

use super::node_widget::NodeWidget;
use super::PanZoom;

pub struct NodeEditor<'a> {
    /// The node widgets
    nodes: Vec<NodeWidget<'a>>,
    /// The offset of each node
    node_positions: Vec<Point>,
    pan_zoom: PanZoom,
}

pub struct NodeEditorState {
    dragging: bool,
    prev_cursor_pos: Option<Point>,
}

impl<'a> NodeEditor<'a> {
    pub fn new(nodes: Vec<NodeWidget<'a>>, node_positions: Vec<Point>, pan_zoom: PanZoom) -> Self {
        Self {
            nodes,
            node_positions,
            pan_zoom,
        }
    }

    fn transformation(&self, top_left: Point) -> Transformation {
        Transformation::identity()
            .translated(-top_left.x, -top_left.y)
            .scaled(1.0 / self.pan_zoom.zoom, 1.0 / self.pan_zoom.zoom)
            .translated(-self.pan_zoom.pan.x, -self.pan_zoom.pan.y)
            .translated(top_left.x, top_left.y)
    }

    fn transform_cursor(&self, cursor_position: Point, top_left: Point) -> Point {
        self.transformation(top_left)
            .transform_point(cursor_position)
    }
}

impl<'a> Widget<BjkUiMessage, BjkUiRenderer> for NodeEditor<'a> {
    fn tag(&self) -> iced_native::widget::tree::Tag {
        WidgetTag::of::<NodeEditorState>()
    }

    fn state(&self) -> iced_native::widget::tree::State {
        WidgetState::new(NodeEditorState {
            dragging: false,
            prev_cursor_pos: None,
        })
    }

    fn width(&self) -> Length {
        Length::Fill
    }

    fn height(&self) -> Length {
        Length::Fill
    }

    fn diff(&self, tree: &mut iced_native::widget::Tree) {
        tree.diff_children_custom(
            &self.nodes,
            |state, node| node.diff(state),
            |node| WidgetTree {
                tag: node.tag(),
                state: node.state(),
                children: node.children(),
            },
        )
    }

    fn children(&self) -> Vec<WidgetTree> {
        self.nodes
            .iter()
            .map(|node| WidgetTree {
                tag: node.tag(),
                state: node.state(),
                children: node.children(),
            })
            .collect_vec()
    }

    fn mouse_interaction(
        &self,
        state: &iced_native::widget::Tree,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
        renderer: &BjkUiRenderer,
    ) -> MouseInteraction {
        let cursor_position = self.transform_cursor(cursor_position, layout.bounds().top_left());
        for ((ch, state), layout) in self
            .nodes
            .iter()
            .zip(state.children.iter())
            .zip(layout.children())
        {
            let interaction =
                ch.mouse_interaction(state, layout, cursor_position, viewport, renderer);
            if interaction != MouseInteraction::Idle {
                return interaction;
            }
        }
        MouseInteraction::Idle
    }

    fn layout(
        &self,
        renderer: &BjkUiRenderer,
        limits: &iced_native::layout::Limits,
    ) -> iced_native::layout::Node {
        let mut children = vec![];
        for (ch, pos) in self.nodes.iter().zip(&self.node_positions) {
            // TODO: Limits: Layout as limitless, but perform some kind of culling?
            let layout = ch.layout(renderer, limits);
            children.push(layout.translate(pos.to_vector()))
        }
        LayoutNode::with_children(limits.max(), children)
    }

    fn draw(
        &self,
        state: &iced_native::widget::Tree,
        renderer: &mut BjkUiRenderer,
        theme: &<BjkUiRenderer as Renderer>::Theme,
        style: &iced_native::renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) {
        let cursor_position = self.transform_cursor(cursor_position, layout.bounds().top_left());

        // Draw the background
        renderer.fill_quad(
            Quad {
                bounds: layout.bounds(),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
            Background::Color(theme.background_dark),
        );

        let top_left = layout.bounds().top_left().to_vector();
        let neg_top_left = Vector::new(0.0, 0.0) - layout.bounds().top_left().to_vector();

        renderer.with_translation(neg_top_left, |renderer| {
            renderer.with_translation(self.pan_zoom.pan, |renderer| {
                renderer.with_scale(self.pan_zoom.zoom, |renderer| {
                    renderer.with_translation(top_left, |renderer| {
                        // Draw the nodes
                        for ((ch, state), layout) in self
                            .nodes
                            .iter()
                            .zip(state.children.iter())
                            .zip(layout.children())
                        {
                            ch.draw(
                                state,
                                renderer,
                                theme,
                                style,
                                layout,
                                cursor_position,
                                viewport,
                            )
                        }
                    });
                });
            });
        });
    }

    fn on_event(
        &mut self,
        state: &mut iced_native::widget::Tree,
        event: iced::Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &BjkUiRenderer,
        clipboard: &mut dyn iced_native::Clipboard,
        shell: &mut iced_native::Shell<'_, BjkUiMessage>,
    ) -> iced::event::Status {
        let contains_cursor = layout.bounds().contains(cursor_position);
        let un_cursor_position = cursor_position;
        let cursor_position = self.transform_cursor(cursor_position, layout.bounds().top_left());

        for ((ch, state), layout) in self
            .nodes
            .iter_mut()
            .zip(state.children.iter_mut())
            .zip(layout.children())
        {
            let status = ch.on_event(
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

        if contains_cursor {
            let state = state.state.downcast_mut::<NodeEditorState>();
            match event {
                Event::Mouse(MouseEvent::ButtonPressed(b)) => {
                    if b == MouseButton::Middle {
                        state.dragging = true;
                        state.prev_cursor_pos = Some(un_cursor_position);
                    }
                }
                Event::Mouse(MouseEvent::ButtonReleased(b)) => {
                    if b == MouseButton::Middle {
                        state.dragging = false;
                    }
                }
                Event::Mouse(MouseEvent::CursorMoved { .. }) => {
                    if state.dragging {
                        let delta = (un_cursor_position - state.prev_cursor_pos.unwrap())
                            * (1.0 / self.pan_zoom.zoom);
                        state.prev_cursor_pos = Some(un_cursor_position);
                        shell.publish(BjkUiMessage::GraphPane(super::GraphPaneMessage::Pan {
                            delta,
                        }));
                    }
                }
                Event::Mouse(MouseEvent::WheelScrolled { delta }) => {
                    let delta = match delta {
                        iced::mouse::ScrollDelta::Lines { y, .. } => y,
                        iced::mouse::ScrollDelta::Pixels { y, .. } => y * 50.0,
                    };

                    self.pan_zoom.adjust_zoom(
                        delta * 0.02,
                        un_cursor_position,
                        layout.bounds().top_left(),
                        0.01,
                        100.0,
                    );

                    shell.publish(BjkUiMessage::GraphPane(super::GraphPaneMessage::Zoom {
                        new_pan_zoom: self.pan_zoom,
                    }))
                }
                _ => {}
            }
        }
        EventStatus::Ignored
    }
}
