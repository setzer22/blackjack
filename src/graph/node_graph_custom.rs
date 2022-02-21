use crate::prelude::*;
use egui::RichText;
use egui_node_graph::{DataTypeTrait, NodeDataTrait, NodeId, NodeResponse, UserResponseTrait};
use rend3::graph::DataHandle;
use serde::{Deserialize, Serialize};

use self::node_templates::GraphNodeType;

pub mod node_templates;

pub struct NodeData {
    op_name: String,
    is_executable: bool,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DataType {
    Vector,
    Scalar,
    Selection,
    Mesh,
    Enum,
    // The path to a (possibly new) file where export contents will be saved to
    NewFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueType {
    /// Used for parameters that can't have a value because they only accept
    /// connections.
    None,
    Vector(Vec3),
    Scalar {
        value: f32,
        min: f32,
        max: f32,
    },
    Selection {
        text: String,
        selection: Option<Vec<u32>>,
    },
    Enum {
        values: Vec<String>,
        selection: Option<u32>,
    },
    NewFile {
        path: Option<std::path::PathBuf>,
    },
}

#[derive(Default)]
pub struct CustomGraphState {
    /// When this option is set by the UI, the side effect encoded by the node
    /// will be executed at the start of the next frame.
    pub run_side_effect: Option<NodeId>,
    /// The currently active node. A program will be compiled to compute the
    /// result of this node and constantly updated in real-time.
    pub active_node: Option<NodeId>,
}

impl DataTypeTrait for DataType {
    fn data_type_color(&self) -> egui::Color32 {
        match self {
            DataType::Mesh => color_from_hex("#266dd3").unwrap(),
            DataType::Vector => color_from_hex("#eecf6d").unwrap(),
            DataType::Scalar => color_from_hex("#eb9fef").unwrap(),
            DataType::Selection => color_from_hex("#4b7f52").unwrap(),
            DataType::Enum => color_from_hex("#ff0000").unwrap(), // Should never be in a port, so highlight in red
            DataType::NewFile => color_from_hex("#ff0000").unwrap(), // Should never be in a port, so highlight in red
        }
    }

    fn name(&self) -> &str {
        match self {
            DataType::Vector => "vector",
            DataType::Scalar => "scalar",
            DataType::Selection => "selection",
            DataType::Mesh => "mesh",
            DataType::Enum => "enum",
            DataType::NewFile => "newfile",
        }
    }
}

pub type Graph = egui_node_graph::Graph<NodeData, DataType, ValueType>;
pub type GraphEditorState = egui_node_graph::GraphEditorState<
    NodeData,
    DataType,
    ValueType,
    GraphNodeType,
    CustomGraphState,
>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomNodeResponse {
    SetActiveNode(NodeId),
    ClearActiveNode,
    RunNodeSideEffect(NodeId),
}

impl UserResponseTrait for CustomNodeResponse {}

impl NodeDataTrait for NodeData {
    type Response = CustomNodeResponse;
    type UserState = CustomGraphState;
    type DataType = DataType;
    type ValueType = ValueType;

    fn bottom_ui(
        &self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        graph: &egui_node_graph::Graph<Self, DataType, ValueType>,
        user_state: &Self::UserState,
    ) -> Vec<egui_node_graph::NodeResponse<Self::Response>>
    where
        Self::Response: egui_node_graph::UserResponseTrait,
    {
        let mut responses = Vec::new();
        ui.horizontal(|ui| {
            // Show 'Enable' button for nodes that output a mesh
            let can_be_enabled = graph[node_id]
                .outputs(graph)
                .any(|output| output.typ == graph::DataType::Mesh);
            let is_active = user_state.active_node == Some(node_id);

            if can_be_enabled {
                ui.horizontal(|ui| {
                    if is_active {
                        if ui.button("👁 Set active").clicked() {
                            responses.push(NodeResponse::User(CustomNodeResponse::SetActiveNode(
                                self.node_id,
                            )));
                        }
                    } else {
                        let button = egui::Button::new(
                            RichText::new("👁 Active").color(egui::Color32::BLACK),
                        )
                        .fill(egui::Color32::GOLD);
                        if ui.add(button).clicked() {
                            responses.push(NodeResponse::User(CustomNodeResponse::ClearActiveNode));
                        }
                    }
                });
            }
            // Show 'Run' button for executable nodes
            if graph[node_id].is_executable() && ui.button("⛭ Run").clicked() {
                responses.push(NodeResponse::User(CustomNodeResponse::RunNodeSideEffect(
                    self.node_id,
                )));
            }
        });
        responses
    }
}
