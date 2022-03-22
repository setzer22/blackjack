use std::collections::HashMap;

use anyhow::*;
use egui_node_graph::NodeTemplateTrait;
use mlua::Table;

use crate::engine::lua_stdlib::Vec3;

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
    name: String,
    label: String,
    inputs: Vec<InputDefinition>,
    outputs: Vec<OutputDefinition>,
}

fn data_type_from_str(s: &str) -> Result<DataType> {
    match s {
        "vec3" => Ok(DataType::Vector),
        "scalar" => Ok(DataType::Scalar),
        "selection" => Ok(DataType::Selection),
        "mesh" => Ok(DataType::Mesh),
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
            DataType::Enum => todo!(),
            DataType::NewFile => todo!(),
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
            label: table.get("label")?,
            inputs,
            outputs,
        })
    }

    pub fn load_nodes_from_table(table: Table) -> Result<HashMap<String, NodeDefinition>> {
        table
            .pairs::<String, Table>()
            .map(|pair| {
                let (k, v) = pair?;
                Ok((k.clone(), NodeDefinition::from_lua(k, v)?))
            })
            .collect()
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
            is_executable: false, // TODO!
        }
    }

    fn build_node(
        &self,
        graph: &mut egui_node_graph::Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        node_id: egui_node_graph::NodeId,
    ) {
        for input in &self.inputs {
            graph.add_input_param(
                node_id,
                input.name.clone(),
                input.data_type,
                input.value.as_ref().unwrap().clone(), // TODO!!
                egui_node_graph::InputParamKind::ConnectionOrConstant, // TODO!!
                true,                                  // TODO!!
            );
        }
        for output in &self.outputs {
            graph.add_output_param(node_id, output.name.clone(), output.data_type);
        }
    }
}
