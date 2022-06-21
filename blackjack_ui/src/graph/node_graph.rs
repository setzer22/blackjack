use crate::prelude::*;
use egui::RichText;
use egui_node_graph::{
    DataTypeTrait, NodeDataTrait, NodeId, NodeResponse, NodeTemplateIter, UserResponseTrait,
    WidgetValueTrait,
};
use serde::{Deserialize, Serialize};

use blackjack_engine::{graph::{DataType, NodeDefinition, NodeDefinitions, ValueType}, prelude::selection::SelectionExpression};

use egui_node_graph::{InputParamKind, NodeTemplateTrait};

pub mod value_widget;

/// A generic egui_node_graph graph, with blackjack-specific parameters
pub type Graph = egui_node_graph::Graph<NodeData, DataTypeUi, ValueTypeUi>;
/// The graph editor state, with blackjack-specific parameters
pub type GraphEditorState = egui_node_graph::GraphEditorState<
    NodeData,
    DataTypeUi,
    ValueTypeUi,
    NodeDefinitionUi,
    CustomGraphState,
>;

/// Blackjack-specific node responses (graph side-effects)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomNodeResponse {
    SetActiveNode(NodeId),
    ClearActiveNode,
    RunNodeSideEffect(NodeId),
}

/// Blackjack-specific global graph state
#[derive(Default, Serialize, Deserialize)]
pub struct CustomGraphState {
    /// When this option is set by the UI, the side effect encoded by the node
    /// will be executed at the start of the next frame.
    pub run_side_effect: Option<NodeId>,
    /// The currently active node. A program will be compiled to compute the
    /// result of this node and constantly updated in real-time.
    pub active_node: Option<NodeId>,
}

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct DataTypeUi(pub DataType); // Prevents orphan rules
impl DataTypeTrait for DataTypeUi {
    fn data_type_color(&self) -> egui::Color32 {
        match self.0 {
            DataType::Mesh => color_from_hex("#266dd3").unwrap(),
            DataType::Vector => color_from_hex("#eecf6d").unwrap(),
            DataType::Scalar => color_from_hex("#eb9fef").unwrap(),
            DataType::Selection => color_from_hex("#4b7f52").unwrap(),
            DataType::String => color_from_hex("#904056").unwrap(),
            DataType::Enum => color_from_hex("#ff0000").unwrap(), // Should never be in a port, so highlight in red
            DataType::NewFile => color_from_hex("#ff0000").unwrap(), // Should never be in a port, so highlight in red
        }
    }

    fn name(&self) -> &str {
        match self.0 {
            DataType::Vector => "vector",
            DataType::Scalar => "scalar",
            DataType::Selection => "selection",
            DataType::Mesh => "mesh",
            DataType::Enum => "enum",
            DataType::NewFile => "newfile",
            DataType::String => "string",
        }
    }
}

impl UserResponseTrait for CustomNodeResponse {}

/// The node data trait can be used to insert a custom UI inside nodes
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeData {
    pub op_name: String,
    pub returns: Option<String>,
    pub is_executable: bool,
}
impl NodeDataTrait for NodeData {
    type Response = CustomNodeResponse;
    type UserState = CustomGraphState;
    type DataType = DataTypeUi;
    type ValueType = ValueTypeUi;

    fn bottom_ui(
        &self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        graph: &egui_node_graph::Graph<Self, DataTypeUi, ValueTypeUi>,
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
                .any(|output| output.typ.0 == DataType::Mesh);
            let is_active = user_state.active_node == Some(node_id);

            if can_be_enabled {
                ui.horizontal(|ui| {
                    if !is_active {
                        if ui.button("👁 Set active").clicked() {
                            responses.push(NodeResponse::User(CustomNodeResponse::SetActiveNode(
                                node_id,
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
            if self.is_executable && ui.button("⛭ Run").clicked() {
                responses.push(NodeResponse::User(CustomNodeResponse::RunNodeSideEffect(
                    node_id,
                )));
            }
        });
        responses
    }
}

pub struct NodeDefinitionsUi<'a>(&'a NodeDefinitions);
impl<'a> NodeTemplateIter for NodeDefinitionsUi<'a> {
    type Item = NodeDefinitionUi;

    fn all_kinds(&self) -> Vec<Self::Item> {
        self.0 .0.values().cloned().map(NodeDefinitionUi).collect()
    }
}

/// Blackjack's custom draw node graph function. It defers to egui_node_graph to
/// draw the graph itself, then interprets any responses it got and applies the
/// required side effects.
pub fn draw_node_graph(ctx: &egui::CtxRef, state: &mut GraphEditorState, defs: &NodeDefinitions) {
    let responses = state.draw_graph_editor(ctx, NodeDefinitionsUi(defs));
    for response in responses.node_responses {
        match response {
            NodeResponse::DeleteNode(node_id) => {
                if state.user_state.active_node == Some(node_id) {
                    state.user_state.active_node = None;
                }
                if state.user_state.run_side_effect == Some(node_id) {
                    state.user_state.run_side_effect = None;
                }
            }
            NodeResponse::User(response) => match response {
                graph::CustomNodeResponse::SetActiveNode(n) => {
                    state.user_state.active_node = Some(n)
                }
                graph::CustomNodeResponse::ClearActiveNode => state.user_state.active_node = None,
                graph::CustomNodeResponse::RunNodeSideEffect(n) => {
                    state.user_state.run_side_effect = Some(n)
                }
            },
            _ => {}
        }
    }
}

#[derive(Clone, Debug)]
pub struct NodeDefinitionUi(pub NodeDefinition);
impl NodeTemplateTrait for NodeDefinitionUi {
    type NodeData = NodeData;
    type DataType = DataTypeUi;
    type ValueType = ValueTypeUi;

    fn node_finder_label(&self) -> &str {
        &self.0.label
    }

    fn node_graph_label(&self) -> String {
        self.0.label.clone()
    }

    fn user_data(&self) -> Self::NodeData {
        NodeData {
            op_name: self.0.name.clone(),
            returns: self.0.returns.clone(),
            is_executable: self.0.executable,
        }
    }

    fn build_node(
        &self,
        graph: &mut egui_node_graph::Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        node_id: egui_node_graph::NodeId,
    ) {
        for input in &self.0.inputs {
            let input_param_kind = match input.data_type {
                DataType::Vector => InputParamKind::ConnectionOrConstant,
                DataType::Scalar => InputParamKind::ConnectionOrConstant,
                DataType::Selection => InputParamKind::ConnectionOrConstant,
                DataType::Mesh => InputParamKind::ConnectionOnly,
                DataType::Enum => InputParamKind::ConstantOnly,
                DataType::NewFile => InputParamKind::ConstantOnly,
                DataType::String => InputParamKind::ConnectionOrConstant,
            };

            graph.add_input_param(
                node_id,
                input.name.clone(),
                DataTypeUi(input.data_type),
                ValueTypeUi(input.value.as_ref().unwrap_or(&ValueType::None).clone()),
                input_param_kind,
                true,
            );
        }
        for output in &self.0.outputs {
            graph.add_output_param(node_id, output.name.clone(), DataTypeUi(output.data_type));
        }
    }
}

/// The widget value trait is used to determine how to display each [`ValueType`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueTypeUi(pub ValueType);
impl WidgetValueTrait for ValueTypeUi {
    fn value_widget(&mut self, param_name: &str, ui: &mut egui::Ui) {
        match &mut self.0 {
            ValueType::Vector(vector) => {
                ui.label(param_name);

                ui.horizontal(|ui| {
                    ui.label("x");
                    ui.add(egui::DragValue::new(&mut vector.x).speed(0.1));
                    ui.label("y");
                    ui.add(egui::DragValue::new(&mut vector.y).speed(0.1));
                    ui.label("z");
                    ui.add(egui::DragValue::new(&mut vector.z).speed(0.1));
                });
            }
            ValueType::Scalar { value, min, max } => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                    ui.add(egui::Slider::new(value, *min..=*max));
                });
            }
            ValueType::Selection { text, selection } => {
                if ui.text_edit_singleline(text).changed() {
                    *selection = SelectionExpression::parse(text).ok();
                }
            }
            ValueType::None => {
                ui.label(param_name);
            }
            ValueType::Enum {
                values,
                selected: selection,
            } => {
                let selected = if let Some(selection) = selection {
                    values[*selection as usize].clone()
                } else {
                    "".to_owned()
                };
                egui::ComboBox::from_label(param_name)
                    .selected_text(selected)
                    .show_ui(ui, |ui| {
                        for (idx, value) in values.iter().enumerate() {
                            ui.selectable_value(selection, Some(idx as u32), value);
                        }
                    });
            }
            ValueType::NewFile { path } => {
                ui.label(param_name);
                ui.horizontal(|ui| {
                    if ui.button("Select").clicked() {
                        *path = rfd::FileDialog::new().save_file();
                    }
                    if let Some(ref path) = path {
                        ui.label(
                            path.clone()
                                .into_os_string()
                                .into_string()
                                .unwrap_or_else(|_| "<Invalid string>".to_owned()),
                        );
                    } else {
                        ui.label("No file selected");
                    }
                });
            }
            ValueType::String { text, multiline } => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                    if *multiline {
                        ui.text_edit_multiline(text);
                    } else {
                        ui.text_edit_singleline(text);
                    }
                });
            }
        }
    }
}
