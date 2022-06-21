use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use egui_node_graph::{InputParamKind, NodeTemplateTrait};
use mlua::Table;

use crate::lua_engine::lua_stdlib::Vec3;

use super::{DataType, ValueType};

#[derive(Clone, Debug)]
pub struct InputDefinition {
    name: String,
    data_type: DataType,
    value: Option<ValueType>,
}

#[derive(Clone, Debug)]
pub struct OutputDefinition {
    name: String,
    data_type: DataType,
}

#[derive(Clone, Debug)]
pub struct NodeDefinition {
    /// The name of the node
    name: String,
    /// The name of the node that will be displayed to users
    label: String,
    /// The definitions for this node's input parameters
    inputs: Vec<InputDefinition>,
    /// The definitions for this node's output parameters
    outputs: Vec<OutputDefinition>,
    /// If present, the output parameter that corresponds to the 'return' value
    /// of this node. The return value of a node can only be a mesh. Nodes that
    /// return something can be "activated", and the returned mesh will be
    /// displayed on the blackjack viewport.
    returns: Option<String>,
    /// Executable nodes can be executed once by pressing a button. This mode of
    /// execution is used for things like file exporters.
    executable: bool,
}

pub struct NodeDefinitions(pub BTreeMap<String, NodeDefinition>);

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
            DataType::Vector => Some(ValueType::Vector(table.get::<_, Vec3>("default")?.0)),
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

impl NodeTemplateTrait for NodeDefinition {
    type NodeData = super::NodeData;
    type DataType = super::DataType;
    type ValueType = super::ValueType;

    fn node_finder_label(&self) -> &str {
        &self.label
    }

    fn node_graph_label(&self) -> String {
        self.label.clone()
    }

    fn user_data(&self) -> Self::NodeData {
        Self::NodeData {
            op_name: self.name.clone(),
            returns: self.returns.clone(),
            is_executable: self.executable,
        }
    }

    fn build_node(
        &self,
        graph: &mut egui_node_graph::Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        node_id: egui_node_graph::NodeId,
    ) {
        for input in &self.inputs {
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
                input.data_type,
                input.value.as_ref().unwrap_or(&ValueType::None).clone(),
                input_param_kind,
                true,
            );
        }
        for output in &self.outputs {
            graph.add_output_param(node_id, output.name.clone(), output.data_type);
        }
    }
}
