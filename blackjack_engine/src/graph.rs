use std::collections::BTreeMap;

use crate::{mesh::halfedge::selection::SelectionExpression, lua_engine::lua_stdlib::LVec3};
use mlua::Table;
use serde::{Serialize, Deserialize};
use anyhow::{anyhow, Result};

/// Blackjack-specific per-node data.
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeData {
    pub op_name: String,
    pub returns: Option<String>,
    pub is_executable: bool,
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
