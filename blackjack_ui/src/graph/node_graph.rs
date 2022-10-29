// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::borrow::Cow;

use crate::custom_widgets::smart_dragvalue::SmartDragValue;
use crate::{application::code_viewer::code_edit_ui, prelude::*};
use egui::RichText;
use egui_node_graph::{
    DataTypeTrait, InputId, NodeDataTrait, NodeId, NodeResponse, NodeTemplateIter,
    UserResponseTrait, WidgetValueTrait,
};
use serde::{Deserialize, Serialize};

use blackjack_engine::{
    graph::{BlackjackValue, DataType, FilePathMode, InputValueConfig, NodeDefinitions},
    prelude::selection::SelectionExpression,
};

use egui_node_graph::{InputParamKind, NodeTemplateTrait};

/// A generic egui_node_graph graph, with blackjack-specific parameters
pub type Graph = egui_node_graph::Graph<NodeData, DataTypeUi, ValueTypeUi>;
/// The graph editor state, with blackjack-specific parameters
pub type GraphEditorState = egui_node_graph::GraphEditorState<
    NodeData,
    DataTypeUi,
    ValueTypeUi,
    NodeOpName,
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
#[derive(Serialize, Deserialize)]
pub struct CustomGraphState {
    /// When this option is set by the UI, the side effect encoded by the node
    /// will be executed at the start of the next frame.
    pub run_side_effect: Option<NodeId>,
    /// The currently active node. A program will be compiled to compute the
    /// result of this node and constantly updated in real-time.
    pub active_node: Option<NodeId>,
    /// A pointer to the node definitions. This is automatically updated when
    /// the node definitions change during hot reload.
    #[serde(skip)]
    pub node_definitions: NodeDefinitions,

    pub promoted_params: HashMap<InputId, String>,
}

impl CustomGraphState {
    pub fn new(node_definitions: NodeDefinitions) -> Self {
        Self {
            node_definitions,
            run_side_effect: None,
            active_node: None,
            promoted_params: HashMap::default(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct DataTypeUi(pub DataType); // Prevents orphan rules
impl DataTypeTrait<CustomGraphState> for DataTypeUi {
    fn data_type_color(&self, _user_state: &mut CustomGraphState) -> egui::Color32 {
        match self.0 {
            DataType::Mesh => color_from_hex("#b43e3e").unwrap(),
            DataType::HeightMap => color_from_hex("#33673b").unwrap(),
            DataType::Vector => color_from_hex("#1A535C").unwrap(),
            DataType::Scalar => color_from_hex("#4ecdc4").unwrap(),
            DataType::Selection => color_from_hex("#f7fff7").unwrap(),
            DataType::String => color_from_hex("#ffe66d").unwrap(),
        }
    }

    fn name(&self) -> Cow<str> {
        Cow::Borrowed(match self.0 {
            DataType::Vector => "vector",
            DataType::Scalar => "scalar",
            DataType::Selection => "selection",
            DataType::Mesh => "mesh",
            DataType::HeightMap => "heightmap",
            DataType::String => "string",
        })
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
        user_state: &mut Self::UserState,
    ) -> Vec<egui_node_graph::NodeResponse<Self::Response, NodeData>>
    where
        Self::Response: egui_node_graph::UserResponseTrait,
    {
        let mut responses = Vec::new();
        ui.horizontal(|ui| {
            // Show 'Enable' button for nodes that output a mesh
            let can_be_enabled = graph[node_id]
                .outputs(graph)
                .any(|output| output.typ.0.can_be_enabled());
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

/// Blackjack's custom draw node graph function. It defers to egui_node_graph to
/// draw the graph itself, then interprets any responses it got and applies the
/// required side effects.
pub fn draw_node_graph(
    ctx: &egui::Context,
    editor_state: &mut GraphEditorState,
    custom_state: &mut CustomGraphState,
    defs: &NodeDefinitions,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let responses =
            editor_state.draw_graph_editor(ui, NodeOpNames(defs.node_names()), custom_state);
        for response in responses.node_responses {
            match response {
                NodeResponse::DeleteNodeFull { node_id, .. } => {
                    if custom_state.active_node == Some(node_id) {
                        custom_state.active_node = None;
                    }
                    if custom_state.run_side_effect == Some(node_id) {
                        custom_state.run_side_effect = None;
                    }
                }
                NodeResponse::User(response) => match response {
                    graph::CustomNodeResponse::SetActiveNode(n) => {
                        custom_state.active_node = Some(n)
                    }
                    graph::CustomNodeResponse::ClearActiveNode => custom_state.active_node = None,
                    graph::CustomNodeResponse::RunNodeSideEffect(n) => {
                        custom_state.run_side_effect = Some(n)
                    }
                },
                _ => {}
            }
        }
    });
}

pub struct NodeOpNames(Vec<String>);
impl NodeTemplateIter for NodeOpNames {
    type Item = NodeOpName;

    fn all_kinds(&self) -> Vec<Self::Item> {
        self.0.iter().cloned().map(NodeOpName).collect()
    }
}

/// Returns the InputParamKind for each of the blackjack data types. This is
/// currently hardcoded and nodes are not allowed to customise it.
pub fn data_type_to_input_param_kind(data_type: DataType) -> InputParamKind {
    match data_type {
        DataType::Vector => InputParamKind::ConnectionOrConstant,
        DataType::Scalar => InputParamKind::ConnectionOrConstant,
        DataType::Selection => InputParamKind::ConnectionOrConstant,
        DataType::Mesh => InputParamKind::ConnectionOnly,
        DataType::HeightMap => InputParamKind::ConnectionOnly,
        DataType::String => InputParamKind::ConnectionOrConstant,
    }
}

/// For now, the "shown inline" property is not customizable and is always set
/// to "true" by default, unless overriden by the user.
pub fn default_shown_inline() -> bool {
    true
}

#[derive(Clone, Debug)]
pub struct NodeOpName(String);
impl NodeTemplateTrait for NodeOpName {
    type NodeData = NodeData;
    type DataType = DataTypeUi;
    type ValueType = ValueTypeUi;
    type UserState = CustomGraphState;

    fn node_finder_label(&self, custom_state: &mut CustomGraphState) -> Cow<str> {
        if let Some(node_def) = custom_state.node_definitions.node_def(&self.0) {
            Cow::Owned(node_def.label.to_string())
        } else {
            Cow::Owned(format!("⚠ {}", self.0))
        }
    }

    fn node_graph_label(&self, custom_state: &mut CustomGraphState) -> String {
        if let Some(node_def) = custom_state.node_definitions.node_def(&self.0) {
            node_def.label.to_string()
        } else {
            format!("⚠ {}", self.0)
        }
    }

    fn user_data(&self, custom_state: &mut CustomGraphState) -> Self::NodeData {
        let node_def = custom_state.node_definitions.node_def(&self.0).expect(
            "This method is only called when creating a new node.\
             Definitions can't be outdated at this point.",
        );
        // TODO: We shouldn't store `returns` and `is_executable` here.
        // Everything can be fetched from the op_name
        NodeData {
            op_name: node_def.op_name.clone(),
            returns: node_def.returns.clone(),
            is_executable: node_def.executable,
        }

        // WIP: Continuing the refactor, I need to make sure we don't `unwrap`
        // the node_def here, unless we can prove that's the case. This is
        // because sometimes we can end up with an outdated node definition.
        //
        // The two cases I just did above are unnecessary: We can guarantee node
        // templates will always be up to date because we regenerate them every
        // frame. The changes I need to do are in ValueTypeTrait.
    }

    fn build_node(
        &self,
        graph: &mut egui_node_graph::Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        custom_state: &mut Self::UserState,
        node_id: egui_node_graph::NodeId,
    ) {
        let node_def = custom_state.node_definitions.node_def(&self.0).unwrap();
        for input in &node_def.inputs {
            let input_param_kind = data_type_to_input_param_kind(input.data_type);

            graph.add_input_param(
                node_id,
                input.name.clone(),
                DataTypeUi(input.data_type),
                ValueTypeUi(match input.config {
                    InputValueConfig::Enum {
                        ref values,
                        default_selection,
                    } => {
                        if let Some(i) = default_selection {
                            BlackjackValue::String(values[i as usize].clone())
                        } else {
                            BlackjackValue::String("".into())
                        }
                    }
                    InputValueConfig::Vector { default } => BlackjackValue::Vector(default),
                    InputValueConfig::Scalar { default, .. } => BlackjackValue::Scalar(default),
                    InputValueConfig::Selection {
                        ref default_selection,
                    } => BlackjackValue::Selection(
                        default_selection.unparse(),
                        Some(default_selection.clone()),
                    ),
                    InputValueConfig::FilePath {
                        ref default_path, ..
                    } => BlackjackValue::String(
                        default_path.as_ref().cloned().unwrap_or_else(|| "".into()),
                    ),
                    InputValueConfig::String {
                        ref default_text, ..
                    } => BlackjackValue::String(default_text.clone()),
                    InputValueConfig::None => BlackjackValue::None,
                    InputValueConfig::LuaString {} => BlackjackValue::String("".into()),
                }),
                input_param_kind,
                default_shown_inline(),
            );
        }
        for output in &node_def.outputs {
            graph.add_output_param(node_id, output.name.clone(), DataTypeUi(output.data_type));
        }
    }
}

/// The widget value trait is used to determine how to display each [`ValueType`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueTypeUi(pub BlackjackValue);

impl Default for ValueTypeUi {
    fn default() -> Self {
        Self(BlackjackValue::None)
    }
}

impl WidgetValueTrait for ValueTypeUi {
    type UserState = CustomGraphState;
    type Response = CustomNodeResponse;
    type NodeData = NodeData;

    fn value_widget(
        &mut self,
        param_name: &str,
        node_id: NodeId,
        ui: &mut egui::Ui,
        user_state: &mut CustomGraphState,
        node_data: &NodeData,
    ) -> Vec<Self::Response> {
        const DRAG_SPEEDS: &[f64] = &[100.0, 10.0, 1.0, 0.1, 0.01, 0.001, 0.0001];
        const DRAG_LABELS: &[&str] = &["100", "10", "1", ".1", ".01", ".001", ".0001"];

        let node_def = user_state
            .node_definitions
            .node_def(&node_data.op_name)
            .unwrap();
        let input_def = node_def
            .inputs
            .iter()
            .find(|i| i.name == param_name)
            .unwrap();

        match (&mut self.0, &input_def.config) {
            (BlackjackValue::Vector(vector), InputValueConfig::Vector { .. }) => {
                ui.label(param_name);
                ui.horizontal(|ui| {
                    ui.label("x");
                    ui.add(
                        SmartDragValue::new(&mut vector.x, DRAG_SPEEDS, DRAG_LABELS)
                            .speed(1.0)
                            .decimals(5),
                    );
                    ui.label("y");
                    ui.add(
                        SmartDragValue::new(&mut vector.y, DRAG_SPEEDS, DRAG_LABELS)
                            .speed(1.0)
                            .decimals(5),
                    );
                    ui.label("z");
                    ui.add(
                        SmartDragValue::new(&mut vector.z, DRAG_SPEEDS, DRAG_LABELS)
                            .speed(1.0)
                            .decimals(5),
                    );
                });
            }
            (
                BlackjackValue::Scalar(value),
                InputValueConfig::Scalar {
                    min,
                    max,
                    soft_min,
                    soft_max,
                    num_decimals,
                    ..
                },
            ) => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                    ui.add(
                        SmartDragValue::new(value, DRAG_SPEEDS, DRAG_LABELS)
                            .speed(1.0)
                            .clamp_range_hard(
                                min.unwrap_or(f32::NEG_INFINITY)..=max.unwrap_or(f32::INFINITY),
                            )
                            .clamp_range_soft(
                                soft_min.unwrap_or(f32::NEG_INFINITY)
                                    ..=soft_max.unwrap_or(f32::INFINITY),
                            )
                            .decimals(num_decimals.unwrap_or(5) as usize),
                    )
                });
            }
            (BlackjackValue::String(string), InputValueConfig::Enum { values, .. }) => {
                egui::ComboBox::from_label(param_name)
                    .selected_text(string.clone())
                    .show_ui(ui, |ui| {
                        for value in values.iter() {
                            ui.selectable_value(string, value.clone(), value);
                        }
                    });
            }
            (BlackjackValue::String(path), InputValueConfig::FilePath { file_path_mode, .. }) => {
                ui.label(param_name);
                ui.horizontal(|ui| {
                    if ui.button("Select").clicked() {
                        let new_path = match file_path_mode {
                            FilePathMode::Open => rfd::FileDialog::new().pick_file(),
                            FilePathMode::Save => rfd::FileDialog::new().save_file(),
                        };

                        if let Some(new_path) = new_path {
                            *path = new_path
                                .into_os_string()
                                .into_string()
                                .unwrap_or_else(|err| format!("INVALID PATH: {err:?}"))
                        }
                    }
                    if !path.is_empty() {
                        ui.label(path.clone());
                    } else {
                        ui.label("No file selected");
                    }
                });
            }
            (BlackjackValue::String(text), InputValueConfig::String { multiline, .. }) => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                    if *multiline {
                        ui.text_edit_multiline(text);
                    } else {
                        ui.text_edit_singleline(text);
                    }
                });
            }
            (BlackjackValue::String(text), InputValueConfig::LuaString {}) => {
                ui.label(param_name);
                code_edit_ui(ui, text);
                //ui.add(egui::TextEdit::multiline(text).text_style(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
            }
            (BlackjackValue::Selection(text, selection), InputValueConfig::Selection { .. }) => {
                if ui.text_edit_singleline(text).changed() {
                    *selection = SelectionExpression::parse(text).ok();
                }
            }
            (BlackjackValue::None, InputValueConfig::None) => {
                ui.label(param_name);
            }
            (a, b) => {
                panic!("Invalid combination {a:?} {b:?}")
            }
        }

        Vec::new()
    }
}
