use crate::prelude::*;
use crate::{lua_engine::lua_stdlib::LVec3, mesh::halfedge::selection::SelectionExpression};
use anyhow::{anyhow, Result};
use egui_node_graph::{
    DataTypeTrait, NodeDataTrait, NodeId, NodeResponse, NodeTemplateIter, UserResponseTrait,
    WidgetValueTrait,
};
use mlua::Table;
use serde::{Deserialize, Serialize};

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

/// Blackjack-specific graph data types.
#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum DataType {
    Vector,
    Scalar,
    Selection,
    Mesh,
    Enum,
    // The path to a (possibly new) file where export contents will be saved to
    NewFile,
    String,
}

/// Blackjack-specific constant types (inline widget)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueType {
    /// Used for parameters that can't have a value because they only accept
    /// connections.
    None,
    Vector(glam::Vec3),
    Scalar {
        value: f32,
        min: f32,
        max: f32,
    },
    Selection {
        text: String,
        selection: Option<SelectionExpression>,
    },
    Enum {
        values: Vec<String>,
        selected: Option<u32>,
    },
    NewFile {
        path: Option<std::path::PathBuf>,
    },
    String {
        multiline: bool,
        text: String,
    },
}

#[derive(Clone, Debug)]
pub struct InputDefinition {
    pub name: String,
    pub data_type: DataType,
    pub value: Option<ValueType>,
}

#[derive(Clone, Debug)]
pub struct OutputDefinition {
    pub name: String,
    pub data_type: DataType,
}

#[derive(Clone, Debug)]
pub struct NodeDefinition {
    /// The name of the node
    pub name: String,
    /// The name of the node that will be displayed to users
    pub label: String,
    /// The definitions for this node's input parameters
    pub inputs: Vec<InputDefinition>,
    /// The definitions for this node's output parameters
    pub outputs: Vec<OutputDefinition>,
    /// If present, the output parameter that corresponds to the 'return' value
    /// of this node. The return value of a node can only be a mesh. Nodes that
    /// return something can be "activated", and the returned mesh will be
    /// displayed on the blackjack viewport.
    pub returns: Option<String>,
    /// Executable nodes can be executed once by pressing a button. This mode of
    /// execution is used for things like file exporters.
    pub executable: bool,
}

fn data_type_from_str(s: &str) -> Result<DataType> {
    match s {
        "vec3" => Ok(DataType::Vector),
        "scalar" => Ok(DataType::Scalar),
        "selection" => Ok(DataType::Selection),
        "mesh" => Ok(DataType::Mesh),
        "enum" => Ok(DataType::Enum),
        "file" => Ok(DataType::NewFile),
        "string" => Ok(DataType::String),
        _ => Err(anyhow!("Invalid datatype in node definition {:?}", s)),
    }
}

impl InputDefinition {
    pub fn from_lua(table: Table) -> Result<Self> {
        let data_type = data_type_from_str(&table.get::<_, String>("type")?)?;
        let value = match data_type {
            DataType::Vector => Some(ValueType::Vector(table.get::<_, LVec3>("default")?.0)),
            DataType::Scalar => Some(ValueType::Scalar {
                value: table.get::<_, f32>("default")?,
                min: table.get::<_, f32>("min")?,
                max: table.get::<_, f32>("max")?,
            }),
            DataType::Selection => Some(ValueType::Selection {
                text: "".into(),
                selection: None,
            }),
            DataType::Mesh => None,
            DataType::Enum => Some(ValueType::Enum {
                values: table
                    .get::<_, Table>("values")?
                    .sequence_values::<String>()
                    .collect::<Result<Vec<_>, _>>()?,
                selected: table.get::<_, Option<u32>>("selected")?,
            }),
            DataType::NewFile => Some(ValueType::NewFile { path: None }),
            DataType::String => Some(ValueType::String {
                text: table.get::<_, String>("default")?,
                multiline: table.get::<_, bool>("multiline")?,
            }),
        };

        Ok(InputDefinition {
            name: table.get("name")?,
            data_type,
            value,
        })
    }
}

impl OutputDefinition {
    pub fn from_lua(table: Table) -> Result<Self> {
        Ok(Self {
            name: table.get("name")?,
            data_type: data_type_from_str(&table.get::<_, String>("type")?)?,
        })
    }
}

impl NodeDefinition {
    pub fn from_lua(name: String, table: Table) -> Result<Self> {
        let inputs = table
            .get::<_, Table>("inputs")?
            .sequence_values()
            .map(|x| InputDefinition::from_lua(x?))
            .collect::<Result<Vec<_>>>()?;

        let outputs = table
            .get::<_, Table>("outputs")?
            .sequence_values()
            .map(|x| OutputDefinition::from_lua(x?))
            .collect::<Result<Vec<_>>>()?;

        Ok(NodeDefinition {
            name,
            inputs,
            outputs,
            label: table.get("label")?,
            returns: table.get::<_, Option<String>>("returns")?,
            executable: table.get::<_, Option<bool>>("executable")?.unwrap_or(false),
        })
    }

    pub fn load_nodes_from_table(table: Table) -> Result<NodeDefinitions> {
        table
            .pairs::<String, Table>()
            .map(|pair| {
                let (k, v) = pair?;
                Ok((k.clone(), NodeDefinition::from_lua(k, v)?))
            })
            .collect::<Result<_>>()
            .map(NodeDefinitions)
    }
}

pub struct NodeDefinitions(pub BTreeMap<String, NodeDefinition>);
