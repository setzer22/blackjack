use blackjack_commons::utils::IteratorUtils;
use blackjack_engine::graph::BjkNodeId;
use blackjack_engine::prelude::Itertools;
use iced_graphics::Transformation;
use iced_native::Renderer;
use slotmap::SecondaryMap;

use crate::graph_editor_pane::node_widget::NodeRow;
use crate::prelude::iced_prelude::*;
use crate::prelude::*;

use super::node_widget::{NodeEventStatus, NodeWidget};
use super::PanZoom;

pub struct NodeEditor<'a> {
    /// The node widgets
    nodes: Vec<NodeWidget<'a>>,
    /// The offset of each node
    node_positions: Vec<Point>,
    pan_zoom: PanZoom,
}

pub struct NodeEditorState {
    panning: bool,
    prev_cursor_pos: Option<Point>,
    // The order of the nodes. Last node will appears on top, and its events will be processed last.
    node_order: Vec<BjkNodeId>,
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

    fn diff_node_order(&self, editor_state: &mut NodeEditorState) {
        // Keep the node order consistent when nodes are created / removed.
        editor_state
            .node_order
            .retain(|node_id| self.nodes.iter().any(|x| x.node_id == *node_id));
        for node in &self.nodes {
            if !editor_state.node_order.contains(&node.node_id) {
                editor_state.node_order.push(node.node_id);
            }
        }
    }
}

impl NodeEditorState {
    pub fn raise_node(&mut self, n: BjkNodeId) {
        self.node_order.retain(|x| *x != n);
        self.node_order.push(n);
    }

    pub fn for_each_node_in_order(
        &self,
        nodes: &[NodeWidget<'_>],
        tree: &WidgetTree,
        layout: Layout,
        reverse: bool,
        mut f: impl FnMut(&NodeWidget<'_>, &WidgetTree, Layout),
    ) {
        let node_id_to_idx: SecondaryMap<BjkNodeId, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.node_id, i))
            .collect();
        let node_layouts: Vec<Layout> = layout.children().collect();
        for node_id in self
            .node_order
            .iter()
            .copied()
            .branch(reverse, |it| it.rev(), |it| it)
        {
            let idx = node_id_to_idx[node_id];
            let node = &nodes[idx];
            let node_state = &tree.children[idx];
            let node_layout = node_layouts[idx];
            f(node, node_state, node_layout);
        }
    }
}

impl<'a> Widget<BjkUiMessage, BjkUiRenderer> for NodeEditor<'a> {
    fn tag(&self) -> iced_native::widget::tree::Tag {
        WidgetTag::of::<NodeEditorState>()
    }

    fn state(&self) -> iced_native::widget::tree::State {
        WidgetState::new(NodeEditorState {
            panning: false,
            prev_cursor_pos: None,
            node_order: self.nodes.iter().map(|n| n.node_id).collect_vec(),
        })
    }

    fn width(&self) -> Length {
        Length::Fill
    }

    fn height(&self) -> Length {
        Length::Fill
    }

    fn diff(&self, tree: &mut iced_native::widget::Tree) {
        let editor_state = tree.state.downcast_mut::<NodeEditorState>();
        self.diff_node_order(editor_state);

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

        let editor_state = state.state.downcast_ref::<NodeEditorState>();

        // Rendering the graph happens within a series of transformations:
        //
        // - The shapes are scaled, around the origin of the node editor. This
        // is achieved by translating and untranslating the view before the
        // scaling.
        //
        // - Additionally, regular pan and zoom are applied.
        let top_left = layout.bounds().top_left().to_vector();
        renderer.with_layer(layout.bounds(), |renderer| {
            renderer.with_translation(top_left.neg(), |renderer| {
                renderer.with_translation(self.pan_zoom.pan, |renderer| {
                    renderer.with_scale(self.pan_zoom.zoom, |renderer| {
                        renderer.with_translation(top_left, |renderer| {
                            // Draw the nodes in the node order. Topmost one is
                            // last, which means we're drawing bottom-to-top
                            editor_state.for_each_node_in_order(
                                &self.nodes,
                                state,
                                layout,
                                false,
                                |node, node_state, node_layout| {
                                    renderer.with_layer(
                                        node_layout.bounds().extend(NodeRow::PORT_RADIUS),
                                        |renderer| {
                                            node.draw(
                                                node_state,
                                                renderer,
                                                theme,
                                                style,
                                                node_layout,
                                                cursor_position,
                                                viewport,
                                            );
                                        },
                                    )
                                },
                            );
                        });
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
        let editor_state = state.state.downcast_mut::<NodeEditorState>();

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
            match status {
                NodeEventStatus::Ignored => {}
                NodeEventStatus::BeingDragged => {
                    editor_state.raise_node(ch.node_id);
                    return EventStatus::Captured;
                }
                NodeEventStatus::CapturedByWidget => {
                    return EventStatus::Captured;
                }
            }
        }

        let mut status = EventStatus::Captured;
        match event {
            Event::Mouse(MouseEvent::ButtonPressed(b)) if contains_cursor => {
                if b == MouseButton::Middle {
                    editor_state.panning = true;
                    editor_state.prev_cursor_pos = Some(un_cursor_position);
                }
            }
            Event::Mouse(MouseEvent::ButtonReleased(b)) => {
                if b == MouseButton::Middle {
                    editor_state.panning = false;
                }
            }
            Event::Mouse(MouseEvent::CursorMoved { .. }) => {
                if editor_state.panning {
                    let delta = (un_cursor_position - editor_state.prev_cursor_pos.unwrap())
                        * (1.0 / self.pan_zoom.zoom);
                    editor_state.prev_cursor_pos = Some(un_cursor_position);
                    shell.publish(BjkUiMessage::GraphPane(super::GraphPaneMessage::Pan {
                        delta,
                    }));
                }
            }
            Event::Mouse(MouseEvent::WheelScrolled { delta }) if contains_cursor => {
                let delta = match delta {
                    iced::mouse::ScrollDelta::Lines { y, .. } => y,
                    iced::mouse::ScrollDelta::Pixels { y, .. } => y * 50.0,
                };

                self.pan_zoom.adjust_zoom(
                    delta * 0.05,
                    un_cursor_position - layout.bounds().top_left().to_vector(),
                    0.25,
                    3.0,
                );

                shell.publish(BjkUiMessage::GraphPane(super::GraphPaneMessage::Zoom {
                    new_pan_zoom: self.pan_zoom,
                }));
            }
            _ => {
                status = EventStatus::Ignored;
            }
        }
        status
    }
}
