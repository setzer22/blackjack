use crate::prelude::graph::*;
use crate::prelude::*;

use super::node_finder::NodeFinder;

#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct PanZoom {
    pub pan: egui::Vec2,
    pub zoom: f32,
}

pub struct GraphEditorState {
    pub graph: Graph,
    /// An ongoing connection interaction: The mouse has dragged away from a
    /// port and the user is holding the click
    pub connection_in_progress: Option<(NodeId, AnyParameterId)>,
    /// The currently active node. A program will be compiled to compute the
    /// result of this node and constantly updated in real-time.
    pub active_node: Option<NodeId>,
    /// The position of each node.
    pub node_positions: HashMap<NodeId, egui::Pos2>,
    /// The node finder is used to create new nodes.
    pub node_finder: Option<NodeFinder>,
    /// When this option is set by the UI, the side effect encoded by the node
    /// will be executed at the start of the next frame.
    pub run_side_effect: Option<NodeId>,
    /// The panning of the graph viewport.
    pub pan_zoom: PanZoom,
}

impl GraphEditorState {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            connection_in_progress: None,
            active_node: None,
            run_side_effect: None,
            node_positions: HashMap::new(),
            node_finder: None,
            pan_zoom: PanZoom {
                pan: egui::Vec2::ZERO,
                zoom: 1.0,
            },
        }
    }
}

impl PanZoom {
    pub fn adjust_zoom(
        &mut self,
        zoom_delta: f32,
        point: egui::Vec2,
        zoom_min: f32,
        zoom_max: f32,
    ) {
        let zoom_clamped = (self.zoom + zoom_delta).clamp(zoom_min, zoom_max);
        let zoom_delta = zoom_clamped - self.zoom;

        self.zoom += zoom_delta;
        self.pan += point * zoom_delta;
    }
}
