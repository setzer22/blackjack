// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::borrow::Cow;

use crate::application::gizmo_ui::UiNodeGizmoStates;
use crate::application::graph_editor::GraphEditor;
use crate::application::serialization;
use crate::custom_widgets::smart_dragvalue::SmartDragValue;
use crate::{application::code_viewer::code_edit_ui, prelude::*};
use blackjack_engine::graph::serialization::SerializedBjkSnippet;
use blackjack_engine::{
    graph::{BlackjackValue, DataType, FilePathMode, InputValueConfig, NodeDefinitions},
    prelude::selection::SelectionExpression,
};
use egui::RichText;
use egui_node_graph::{
    DataTypeTrait, InputId, NodeDataTrait, NodeId, NodeResponse, NodeTemplateIter,
    UserResponseTrait, WidgetValueTrait,
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
    LockGizmos(NodeId),
    UnlockGizmos(NodeId),
}

/// Blackjack-specific global graph state
pub struct CustomGraphState {
    /// When this option is set by the UI, the side effect encoded by the node
    /// will be executed at the start of the next frame.
    pub run_side_effect: Option<NodeId>,
    /// The currently active node. A program will be compiled to compute the
    /// result of this node and constantly updated in real-time.
    pub active_node: Option<NodeId>,
    /// A pointer to the node definitions. This is automatically updated when
    /// the node definitions change during hot reload.
    pub node_definitions: NodeDefinitions,

    pub promoted_params: HashMap<InputId, String>,

    pub gizmo_states: UiNodeGizmoStates,
}

impl CustomGraphState {
    pub fn new(node_definitions: NodeDefinitions, gizmo_states: UiNodeGizmoStates) -> Self {
        Self {
            node_definitions,
            run_side_effect: None,
            active_node: None,
            promoted_params: HashMap::default(),
            gizmo_states,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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
#[derive(Clone)]
pub struct NodeData {
    pub op_name: String,
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
        let node_def = user_state
            .node_definitions
            .node_def(&graph[node_id].user_data.op_name);
        if node_def.is_none() {
            ui.label("⚠ no node definition")
                .on_hover_text("This node is referencing a node definition that doesn't exist.");
            return Default::default();
        }
        let node_def = node_def.unwrap();

        let mut responses = Vec::new();
        ui.horizontal(|ui| {
            // Show 'Enable' button for nodes that output a mesh
            let can_be_enabled = graph[node_id]
                .outputs(graph)
                .any(|output| output.typ.0.can_be_enabled());
            let is_active = user_state.active_node == Some(node_id);

            ui.horizontal(|ui| {
                if can_be_enabled {
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
                }
                if node_def.has_gizmo {
                    if user_state.gizmo_states.is_node_locked(node_id) {
                        let button =
                            egui::Button::new(RichText::new("↺ Gizmo").color(egui::Color32::BLACK))
                                .fill(egui::Color32::GOLD);
                        if ui.add(button).clicked() {
                            responses.push(NodeResponse::User(CustomNodeResponse::UnlockGizmos(
                                node_id,
                            )))
                        }
                    } else if ui.button("↺ Gizmo").clicked() {
                        responses.push(NodeResponse::User(CustomNodeResponse::LockGizmos(node_id)))
                    }
                }
                // Show 'Run' button for executable nodes
                if node_def.executable && ui.button("⛭ Run").clicked() {
                    responses.push(NodeResponse::User(CustomNodeResponse::RunNodeSideEffect(
                        node_id,
                    )));
                }
            });
        });
        responses
    }
}

/// Blackjack's custom draw node graph function. It defers to egui_node_graph to
/// draw the graph itself, then interprets any responses it got and applies the
/// required side effects.
pub fn draw_node_graph(graph_editor: &mut GraphEditor) {
    let GraphEditor {
        editor_state,
        custom_state,
        egui_context: ctx,
        mouse_over_node_finder,
        previous_clipboard_contents,
        pending_paste_operation,
        skip_pending_paste_check,
        ..
    } = graph_editor;
    egui::CentralPanel::default().show(ctx, |ui| {
        // We clone the old graph here, so we can get a hold of the old state
        // before the graph is mutated. This is useful on some operations.
        let old_graph = editor_state.graph.clone();

        let responses = editor_state.draw_graph_editor(
            ui,
            NodeOpNames(custom_state.node_definitions.node_names()),
            custom_state,
        );

        // Store whether the mouse is in the node finder. This helps prevent
        // scroll wheel events.
        *mouse_over_node_finder = responses.cursor_in_finder;

        for response in responses.node_responses {
            match response {
                NodeResponse::DeleteNodeFull { node_id, .. } => {
                    if custom_state.active_node == Some(node_id) {
                        custom_state.active_node = None;

                        // Heuristic: Look for the previous node connected to
                        // the one that got deleted, and activate it.
                        let old_node = &old_graph.nodes[node_id];
                        for (_, input) in &old_node.inputs {
                            if let Some(output_id) = old_graph.connection(*input) {
                                let output = old_graph.get_output(output_id);
                                if output.typ.0.can_be_enabled() {
                                    custom_state.active_node =
                                        Some(old_graph.get_output(output_id).node);
                                    break;
                                }
                            }
                        }
                    }
                    if custom_state.run_side_effect == Some(node_id) {
                        custom_state.run_side_effect = None;
                    }
                    custom_state.gizmo_states.node_deleted(node_id);
                }
                NodeResponse::User(response) => match response {
                    graph::CustomNodeResponse::SetActiveNode(n) => {
                        if let Some(prev_active) = custom_state.active_node {
                            custom_state.gizmo_states.node_left_active(prev_active);
                        }
                        custom_state.active_node = Some(n);
                        // When the active node changes, we want to clear the
                        // existing gizmos referring to the previous node
                        custom_state.gizmo_states.node_is_active(n);
                    }
                    graph::CustomNodeResponse::ClearActiveNode => {
                        if let Some(prev_active) = custom_state.active_node {
                            custom_state.gizmo_states.node_left_active(prev_active);
                        }
                        custom_state.active_node = None;
                    }
                    graph::CustomNodeResponse::RunNodeSideEffect(n) => {
                        custom_state.run_side_effect = Some(n)
                    }
                    CustomNodeResponse::LockGizmos(n) => {
                        custom_state.gizmo_states.lock_gizmos_for(n);
                    }
                    CustomNodeResponse::UnlockGizmos(n) => {
                        custom_state
                            .gizmo_states
                            .unlock_gizmos_for(n, custom_state.active_node);
                    }
                },
                _ => {}
            }
        }

        if ui.input().key_released(egui::Key::C)
            && ui.input().modifiers.ctrl
            && !editor_state.selected_nodes.is_empty()
        {
            match serialization::to_clipboard(
                editor_state,
                custom_state,
                &editor_state.selected_nodes,
            ) {
                Ok(clipboard_data) => {
                    *previous_clipboard_contents = clipboard_data.clone();
                    ui.output().copied_text = clipboard_data;
                }
                Err(err) => {
                    println!("Error: Could not generate clipboard data {err:?}");
                }
            }
        }

        let input = ui.input();
        let cursor_pos = ui.input().pointer.hover_pos().unwrap_or(egui::Pos2::ZERO);
        let mut do_paste = |snippet: SerializedBjkSnippet| {
            if let Err(err) =
                serialization::from_clipboard(editor_state, custom_state, snippet, cursor_pos)
            {
                println!("Error: Could not paste clipboard data: {err:?}")
            }
        };

        if let Some(paste_contents) = input.events.iter().find_map(|ev| match ev {
            egui::Event::Paste(text) => Some(text),
            _ => None,
        }) {
            if let Ok(snippet) = serialization::parse_clipboard_snippet(paste_contents) {
                if previous_clipboard_contents != paste_contents && !*skip_pending_paste_check {
                    *pending_paste_operation = Some(snippet);
                } else {
                    do_paste(snippet);
                }
            } else {
                println!("Tried to paste an invalid snippet.");
            }
        }

        // Do not borrow the egui context for too long or we will deadlock.
        drop(input);

        let mut clear_pending_paste = false;
        if let Some(pending_paste) = pending_paste_operation {
            egui::Window::new("⚠ Warning ⚠")
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| {
                    ui.label(
                        r#"
Remember: When pasting from outside sources, always make sure you trust the author of the snippet.

Pasted nodes can potentially run code, but only when you activate them.
"#,
                    );

                    ui.checkbox(skip_pending_paste_check, "Do not remind me again");

                    ui.horizontal(|ui| {
                        if ui.button("I understand").clicked() {
                            do_paste(std::mem::take(pending_paste));
                            clear_pending_paste = true;
                        }
                        if ui.button("Nevermind").clicked() {
                            clear_pending_paste = true;
                        }
                    });
                });
        }
        if clear_pending_paste {
            *pending_paste_operation = None;
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
        let node_def = custom_state.node_definitions.node_def(&self.0).expect(
            "This method is only called when creating a new node.\
             Definitions can't be outdated at this point.",
        );
        Cow::Owned(node_def.label.to_string())
    }

    fn node_graph_label(&self, custom_state: &mut CustomGraphState) -> String {
        let node_def = custom_state.node_definitions.node_def(&self.0).expect(
            "This method is only called when creating a new node.\
             Definitions can't be outdated at this point.",
        );
        node_def.label.clone()
    }

    fn user_data(&self, custom_state: &mut CustomGraphState) -> Self::NodeData {
        let node_def = custom_state.node_definitions.node_def(&self.0).expect(
            "This method is only called when creating a new node.\
             Definitions can't be outdated at this point.",
        );
        NodeData {
            op_name: node_def.op_name.clone(),
        }
    }

    fn build_node(
        &self,
        graph: &mut egui_node_graph::Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        custom_state: &mut Self::UserState,
        node_id: egui_node_graph::NodeId,
    ) {
        let node_def = custom_state.node_definitions.node_def(&self.0).expect(
            "This method is only called when creating a new node.\
             Definitions can't be outdated at this point.",
        );
        for input in &node_def.inputs {
            let input_param_kind = data_type_to_input_param_kind(input.data_type);

            graph.add_input_param(
                node_id,
                input.name.clone(),
                DataTypeUi(input.data_type),
                ValueTypeUi(input.default_value()),
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
#[derive(Debug, Clone)]
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
        _node_id: NodeId,
        ui: &mut egui::Ui,
        user_state: &mut CustomGraphState,
        node_data: &NodeData,
    ) -> Vec<Self::Response> {
        const FLOAT_DRAG_SPEEDS: &[f64] = &[100.0, 10.0, 1.0, 0.1, 0.01, 0.001, 0.0001];
        const FLOAT_DRAG_LABELS: &[&str] = &["100", "10", "1", ".1", ".01", ".001", ".0001"];
        const INT_DRAG_SPEEDS: &[f64] = &[100.0, 10.0, 1.0];
        const INT_DRAG_LABELS: &[&str] = &["100", "10", "1"];

        let node_def = user_state.node_definitions.node_def(&node_data.op_name);
        let input_def = node_def
            .as_deref()
            .and_then(|d| d.inputs.iter().find(|i| i.name == param_name));

        // This may happen on rare occasions when the nodes are reloaded and a
        // parameter that previously existed now doesn't anymore.
        if input_def.is_none() {
            ui.label("⚠ not found")
                .on_hover_text("This node is referencing a parameter that doesn't exist.");
            return Default::default();
        }
        let input_def = input_def.unwrap();

        match (&mut self.0, &input_def.config) {
            (BlackjackValue::Vector(vector), InputValueConfig::Vector { .. }) => {
                ui.label(param_name);
                ui.horizontal(|ui| {
                    ui.label("x");
                    ui.add(
                        SmartDragValue::new(&mut vector.x, FLOAT_DRAG_SPEEDS, FLOAT_DRAG_LABELS)
                            .speed(1.0)
                            .decimals(5),
                    );
                    ui.label("y");
                    ui.add(
                        SmartDragValue::new(&mut vector.y, FLOAT_DRAG_SPEEDS, FLOAT_DRAG_LABELS)
                            .speed(1.0)
                            .decimals(5),
                    );
                    ui.label("z");
                    ui.add(
                        SmartDragValue::new(&mut vector.z, FLOAT_DRAG_SPEEDS, FLOAT_DRAG_LABELS)
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
                let is_int = *num_decimals == Some(0);
                let drag_speeds = if is_int {
                    INT_DRAG_SPEEDS
                } else {
                    FLOAT_DRAG_SPEEDS
                };
                let drag_labels = if is_int {
                    INT_DRAG_LABELS
                } else {
                    FLOAT_DRAG_LABELS
                };
                let mut drag_value = SmartDragValue::new(value, drag_speeds, drag_labels)
                    .speed(1.0)
                    .clamp_range_hard(
                        min.unwrap_or(f32::NEG_INFINITY)..=max.unwrap_or(f32::INFINITY),
                    )
                    .clamp_range_soft(
                        soft_min.unwrap_or(f32::NEG_INFINITY)..=soft_max.unwrap_or(f32::INFINITY),
                    )
                    .decimals(num_decimals.unwrap_or(5) as usize);
                if is_int {
                    drag_value = drag_value.default_range_index(2);
                }

                ui.horizontal(|ui| {
                    ui.label(param_name);
                    ui.add(drag_value)
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
                if *multiline {
                    ui.label(param_name);
                }
                ui.horizontal(|ui| {
                    if !multiline {
                        ui.label(param_name);
                    }
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
