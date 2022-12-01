use blackjack_engine::graph::{BjkGraph, BjkNodeId, DataType};
use glam::Vec2;
use slotmap::SecondaryMap;

use crate::prelude::iced_prelude::*;
use crate::prelude::*;

pub mod node_editor;
pub mod node_widget;
pub mod port_widget;

#[derive(Debug, Clone)]
pub enum GraphPaneMessage {
    NodeMoved { node_id: BjkNodeId, delta: Vector },
    Zoom { zoom_delta: f32, point: Vector },
    Pan { delta: Vector },
}

#[derive(Default)]
pub struct GraphEditorPane;

pub struct GraphEditorState {
    graph: BjkGraph,
    node_positions: SecondaryMap<BjkNodeId, Vec2>,
    pan_zoom: PanZoom,
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PanZoom {
    pub pan: Vector,
    pub zoom: f32,
}

impl PanZoom {
    pub fn adjust_zoom(&mut self, zoom_delta: f32, point: Vector, zoom_min: f32, zoom_max: f32) {
        let zoom_clamped = (self.zoom + zoom_delta).clamp(zoom_min, zoom_max);
        let zoom_delta = zoom_clamped - self.zoom;

        self.zoom += zoom_delta;
        self.pan = self.pan + point * zoom_delta;
    }
}

impl Default for GraphEditorState {
    fn default() -> Self {
        let mut graph = BjkGraph::new();
        let mut node_positions = SecondaryMap::new();

        let node1 = graph.add_node("Potato", None);
        graph
            .add_input(node1, "foo", DataType::Scalar, None)
            .unwrap();
        graph
            .add_input(node1, "bar", DataType::Scalar, None)
            .unwrap();
        graph
            .add_input(node1, "baz", DataType::Scalar, None)
            .unwrap();
        graph
            .add_output(node1, "foo_out", DataType::Scalar)
            .unwrap();
        node_positions.insert(node1, glam::Vec2::new(100.0, 100.0));

        let node2 = graph.add_node("Other node", None);
        graph
            .add_input(node2, "afoo", DataType::Scalar, None)
            .unwrap();
        graph
            .add_input(node2, "abar", DataType::Scalar, None)
            .unwrap();
        graph
            .add_output(node2, "afoo1_out", DataType::Scalar)
            .unwrap();
        graph
            .add_output(node2, "afoo2_out", DataType::Scalar)
            .unwrap();
        node_positions.insert(node2, glam::Vec2::new(200.0, 200.0));

        Self {
            graph,
            node_positions,
            pan_zoom: PanZoom {
                pan: Vector::new(0.0, 0.0),
                zoom: 1.0,
            },
        }
    }
}

impl GraphEditorPane {
    pub fn new() -> Self {
        Self {}
    }

    pub fn titlebar_view(&self, _graph: &GraphEditorState) -> BjkUiElement<'_> {
        row(vec![
            text("Graph editor").into(),
            h_spacer().into(),
            button("Close").into(),
        ])
        .into()
    }

    pub fn content_view(&self, graph: &GraphEditorState) -> BjkUiElement<'_> {
        let mut node_widgets = vec![];
        let mut node_widget_positions = vec![];

        for (node_id, node) in &graph.graph.nodes {
            let pos = graph.node_positions[node_id];
            node_widget_positions.push(pos.to_iced_point());

            let mut rows = vec![];
            for input in &node.inputs {
                rows.push(node_widget::NodeRow::input(
                    text(&input.name),
                    Color::from_rgb8(42, 72, 92),
                ));
            }
            for output in &node.outputs {
                rows.push(node_widget::NodeRow::output(
                    text(&output.name),
                    Color::from_rgb8(42, 72, 92),
                ));
            }

            let node_widget = node_widget::NodeWidget {
                node_id,
                // TODO: Use label, not op name. This requires node definitions
                titlebar_left: container(text(&node.op_name)).padding(4).into(),
                titlebar_right: container(text("x")).padding(4).into(),
                rows,
                bottom_ui: button("Set Active").into(),
                v_separation: 4.0,
                h_separation: 12.0,
                extra_v_separation: 3.0,
            };
            node_widgets.push(node_widget);
        }

        let editor = node_editor::NodeEditor::new(node_widgets.into_iter(), node_widget_positions, graph.pan_zoom);
        dbg!(graph.pan_zoom);

        container(BjkUiElement::new(editor)).padding(3).into()
    }
}

impl GraphEditorState {
    pub fn update(&mut self, msg: GraphPaneMessage) {
        match msg {
            GraphPaneMessage::NodeMoved { node_id, delta } => {
                self.node_positions[node_id] += delta.to_glam();
            }
            GraphPaneMessage::Zoom { zoom_delta, point } => {
                const ZOOM_MIN: f32 = 0.05;
                const ZOOM_MAX: f32 = 100.0;
                self.pan_zoom
                    .adjust_zoom(zoom_delta, point, ZOOM_MIN, ZOOM_MAX)
            }
            GraphPaneMessage::Pan { delta } => {
                self.pan_zoom.pan = self.pan_zoom.pan + delta;
            }
        }
    }
}
