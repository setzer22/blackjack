use blackjack_commons::utils::IteratorUtils;
use blackjack_engine::graph::{BjkGraph, BjkNodeId, DataType, NodeDefinitions};
use glam::Vec2;
use iced_native::overlay;
use slotmap::SecondaryMap;

use crate::helper_widgets::overlap_container;
use crate::prelude::iced_prelude::*;
use crate::prelude::*;

pub mod node_editor;
pub mod node_widget;

#[derive(Debug, Clone)]
pub enum GraphPaneMessage {
    NodeMoved { node_id: BjkNodeId, delta: Vector },
    Zoom { new_pan_zoom: PanZoom },
    Pan { delta: Vector },
}

#[derive(Default)]
pub struct GraphEditorPane;

pub struct GraphEditorState {
    graph: BjkGraph,
    node_positions: SecondaryMap<BjkNodeId, Vec2>,
    node_definitions: NodeDefinitions,
    pan_zoom: PanZoom,
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PanZoom {
    pub pan: Vector,
    pub zoom: f32,
}

impl PanZoom {
    /// Increments the current zoom by zoom_level, and increases the current
    /// zoom level by `zoom_delta` and adjusts the panning so that zoom is
    /// centered around the given `point`.
    ///
    /// The point is provided in window-space coordinates, relative to the
    /// top-left corner of the graph.
    pub fn adjust_zoom(&mut self, zoom_delta: f32, point: Point, zoom_min: f32, zoom_max: f32) {
        // Adjust the zoom level, taking min / max into account.
        let zoom_new = {
            let clamped = (self.zoom + zoom_delta).clamp(zoom_min, zoom_max);
            let delta_clamped = clamped - self.zoom;
            self.zoom * (1.0 + delta_clamped)
        };

        // To adjust the pan, we consider the point at the previous zoom level,
        // and the position where that point ends up after modifying the zoom
        // level if we didn't correct the pan. We then shift the view in the
        // opposite direction to keep that point at the same position.
        //
        // NOTE: The points at current and new zoom level are obtained by
        // dividing the cursor position by the zoom. Division is done to apply
        // the inverse transformation, since we are converting from screen space
        // to graph space, not vice-versa. We ignore pan in the transformation
        // because we're only interested in the difference.
        let point = point.to_vector();
        let pan_correction = point.div(zoom_new) - point.div(self.zoom);

        self.pan = self.pan + pan_correction;
        self.zoom = zoom_new;
    }
}

impl GraphEditorPane {
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
            let node_def = graph.node_definitions.node_def(&node.op_name);

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

            let title = node_def
                .as_ref()
                .map(|x| &x.label)
                .unwrap_or_else(|| &node.op_name);

            let node_widget = node_widget::NodeWidget {
                node_id,
                titlebar_left: container(text(title)).padding(4).into(),
                titlebar_right: container(text("x")).padding(4).into(),
                rows,
                bottom_ui: button("Set Active").into(),
                v_separation: 4.0,
                h_separation: 12.0,
                extra_v_separation: 3.0,
            };
            node_widgets.push(node_widget);
        }

        let editor =
            node_editor::NodeEditor::new(node_widgets, node_widget_positions, graph.pan_zoom);

        BjkUiElement::new(overlap_container::OverlapContainer {
            bg_widget: container(BjkUiElement::new(editor)).padding(3).into(),
            fg_widget: node_picker(&graph.node_definitions),
            fg_widget_position: Vector::new(100.0, 100.0),
        })
    }
}

impl GraphEditorState {
    pub fn new(node_definitions: NodeDefinitions) -> Self {
        Self {
            graph: BjkGraph::new(),
            node_positions: SecondaryMap::new(),
            node_definitions,
            pan_zoom: PanZoom {
                pan: Vector::new(0.0, 0.0),
                zoom: 1.0,
            },
        }
    }

    pub fn spawn_node(&mut self, op_name: &str, position: glam::Vec2) -> Result<BjkNodeId> {
        let node_id = self.graph.spawn_node(op_name, &self.node_definitions)?;
        self.node_positions.insert(node_id, position);
        Ok(node_id)
    }

    pub fn update(&mut self, msg: GraphPaneMessage) {
        match msg {
            GraphPaneMessage::NodeMoved { node_id, delta } => {
                self.node_positions[node_id] += delta.to_glam();
            }
            GraphPaneMessage::Zoom { new_pan_zoom } => {
                self.pan_zoom = new_pan_zoom;
            }
            GraphPaneMessage::Pan { delta } => {
                self.pan_zoom.pan = self.pan_zoom.pan + delta;
            }
        }
    }
}

pub fn node_picker<'a>(node_definitions: &NodeDefinitions) -> BjkUiElement<'a> {
    let buttons = node_definitions
        .node_names()
        .into_iter()
        // TODO: Dummy
        .map(|x| {
            button(&x)
                .on_press(BjkUiMessage::Dummy)
                .width(Length::Fill)
                .into()
        })
        .collect();

    container(column(Vec::from([
        text_input("potato", "testo", |_| BjkUiMessage::Dummy).into(),
        v_scroll_area(container(column(buttons)).padding(2)).into(),
    ])))
    .max_width(300)
    .style(BjkContainerStyle::NodePicker)
    .into()
}
