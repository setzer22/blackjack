// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::BTreeMap;

use crate::prelude::*;
use crate::{lua_engine::lua_stdlib::LVec3, mesh::halfedge::selection::SelectionExpression};
use anyhow::{anyhow, Result};
use mlua::{Table, ToLua};
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

pub struct LuaExpression(pub String);

/// A node has inputs (dependencies) that need to be met. A dependency can be
/// met in three different ways.
pub enum DependencyKind {
    /// Executing an arbitrary lua expression (computed).
    Computed(LuaExpression),
    /// Taking the value of an external parameter, from the inputs to the graph
    /// function itself.
    External,
    /// Taking the value from another node's outputs.
    Connection { node: BjkNodeId, param_name: String },
}

/// The data types available for graph parameters
#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum DataType {
    Vector,
    Scalar,
    Selection,
    Mesh,
    String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlackjackValue {
    Vector(glam::Vec3),
    Scalar(f32),
    String(String),
    Selection(String, Option<SelectionExpression>),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackjackParameter {
    pub value: BlackjackValue,
    pub config: InputValueConfig,
    /// Stores the name of the promoted parameter, if this parameter is
    /// promoted. A promoted parameter will be visible on external editors, such
    /// as the ones offered by game engine integrations.
    pub promoted_name: Option<String>,
}

impl<'lua> ToLua<'lua> for BlackjackValue {
    fn to_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        match self {
            BlackjackValue::Vector(v) => Ok(v.cast_to_lua(lua)),
            BlackjackValue::Scalar(s) => Ok(s.cast_to_lua(lua)),
            BlackjackValue::String(s) => s.to_lua(lua),
            BlackjackValue::Selection(_, sel) => sel.to_lua(lua),
            BlackjackValue::None => Ok(mlua::Value::Nil),
        }
    }
}

/// An input parameter in the graph. Inputs represent data dependencies that
/// need to be met before executing a node.
pub struct InputParameter {
    pub name: String,
    pub data_type: DataType,
    pub kind: DependencyKind,
}

/// An output parameter. Outputs are pieces of data produced by a node, which
/// can be used to feed into another nodes as inputs.
pub struct Output {
    pub name: String,
    pub data_type: DataType,
}

/// A node in the blackjack graph
pub struct BjkNode {
    pub op_name: String,
    pub inputs: Vec<InputParameter>,
    pub outputs: Vec<Output>,
}

slotmap::new_key_type! { pub struct BjkNodeId; }
impl BjkNodeId {
    pub fn display_id(self) -> String {
        format!("{:?}", self.0)
    }
}

/// The blackjack graph data structure. This is the main data model describing a
/// blackjack procedural asset, or 'Jack'. Graphs describe a computation to be
/// performed by applying transformations (nodes) over data (input/output
/// parameters).
#[derive(Default)]
pub struct BjkGraph {
    pub nodes: SlotMap<BjkNodeId, BjkNode>,
}

/// The settings to describe an input value in a node template. This information
/// is used by the UI, or engine integrations, to know which default values
/// should be displayed in widgets when no other value is provided.
///
/// The variants in this structure will typically offer more information than
/// the value itself, such as the limits for that parameter or other useful
/// validation information. There is not a 1:1 correspondence between data types
/// and config variants. Some variants (e.g. `Enum`, `FilePath`) are special cases
/// of some datatype (i.e. `String`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputValueConfig {
    Vector {
        default: glam::Vec3,
    },
    Scalar {
        default: f32,
        min: f32,
        max: f32,
    },
    Selection {
        default_selection: SelectionExpression,
    },
    Enum {
        values: Vec<String>,
        default_selection: Option<u32>,
    },
    FilePath {
        default_path: Option<String>,
    },
    String {
        multiline: bool,
        default_text: String,
    },
    LuaString {},
    None,
}

/// The definition of an input parameter inside the node library.
#[derive(Clone, Debug)]
pub struct InputDefinition {
    pub name: String,
    pub data_type: DataType,
    pub config: InputValueConfig,
}

/// The definition of an output parameter inside the node library
#[derive(Clone, Debug)]
pub struct OutputDefinition {
    pub name: String,
    pub data_type: DataType,
}

/// A node definition inside the node library
#[derive(Clone, Debug)]
pub struct NodeDefinition {
    /// The name of the node, as registered in NodeLibraries
    pub op_name: String,
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

/// A collection of node definitions. This struct is the Rust counterpart to the
/// node library in Lua.
pub struct NodeDefinitions(pub BTreeMap<String, NodeDefinition>);

/// Given a string representing an input definition type (taken from a Lua
/// file), returns the data type for that parameter.
fn data_type_from_str(s: &str) -> Result<DataType> {
    match s {
        "vec3" => Ok(DataType::Vector),
        "scalar" => Ok(DataType::Scalar),
        "selection" => Ok(DataType::Selection),
        "mesh" => Ok(DataType::Mesh),
        "enum" => Ok(DataType::String),
        "file" => Ok(DataType::String),
        "string" => Ok(DataType::String),
        "lua_string" => Ok(DataType::String),
        _ => Err(anyhow!("Invalid datatype in node definition {:?}", s)),
    }
}

impl InputDefinition {
    /// Parses from a Lua table describing this [`InputDefinition`]
    pub fn from_lua(table: Table) -> Result<Self> {
        let type_str: String = table.get::<_, String>("type")?;
        let data_type = data_type_from_str(&type_str)?;
        let value = match data_type {
            DataType::Vector => InputValueConfig::Vector {
                default: table.get::<_, LVec3>("default")?.0,
            },
            DataType::Scalar => InputValueConfig::Scalar {
                default: table.get::<_, f32>("default")?,
                min: table.get::<_, f32>("min")?,
                max: table.get::<_, f32>("max")?,
            },
            DataType::Selection => InputValueConfig::Selection {
                default_selection: SelectionExpression::None,
            },
            DataType::Mesh => InputValueConfig::None,
            DataType::String if type_str == "enum" => InputValueConfig::Enum {
                values: table
                    .get::<_, Table>("values")?
                    .sequence_values::<String>()
                    .collect::<Result<Vec<_>, _>>()?,
                default_selection: table.get::<_, Option<u32>>("selected")?,
            },
            DataType::String if type_str == "file" => {
                InputValueConfig::FilePath { default_path: None }
            }
            DataType::String if type_str == "lua_string" => InputValueConfig::LuaString {},
            DataType::String => InputValueConfig::String {
                default_text: table.get::<_, String>("default")?,
                multiline: table.get::<_, bool>("multiline")?,
            },
        };

        Ok(InputDefinition {
            name: table.get("name")?,
            data_type,
            config: value,
        })
    }
}

impl OutputDefinition {
    /// Parses from a Lua table describing this [`OutputDefinition`]
    pub fn from_lua(table: Table) -> Result<Self> {
        Ok(Self {
            name: table.get("name")?,
            data_type: data_type_from_str(&table.get::<_, String>("type")?)?,
        })
    }
}

impl NodeDefinition {
    /// Parses from a Lua table describing this [`NodeDefinition`]
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
            op_name: name,
            inputs,
            outputs,
            label: table.get("label")?,
            returns: table.get::<_, Option<String>>("returns")?,
            executable: table.get::<_, Option<bool>>("executable")?.unwrap_or(false),
        })
    }

    /// Loads a group of [`NodeDefinitions`] from a Lua table
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

impl BjkGraph {
    // Constructs an empty graph
    pub fn new() -> Self {
        Self {
            nodes: Default::default(),
        }
    }
    /// Adds a new empty node to the graph
    pub fn add_node(&mut self, op_name: impl ToString) -> BjkNodeId {
        self.nodes.insert(BjkNode {
            op_name: op_name.to_string(),
            inputs: vec![],
            outputs: vec![],
        })
    }

    /// Registers a new input for `node_id`
    pub fn add_input(
        &mut self,
        node_id: BjkNodeId,
        name: impl ToString,
        data_type: DataType,
    ) -> Result<()> {
        let name = name.to_string();
        let node = &mut self.nodes[node_id];
        if node.inputs.iter().any(|input| input.name == name) {
            bail!("Input parameter {name} already exists for node {node_id:?}");
        } else {
            self.nodes[node_id].inputs.push(InputParameter {
                name,
                data_type,
                kind: DependencyKind::External,
            });
        }
        Ok(())
    }

    /// Registers a new output for `node_id`
    pub fn add_output(
        &mut self,
        node_id: BjkNodeId,
        name: impl ToString,
        data_type: DataType,
    ) -> Result<()> {
        let name = name.to_string();
        let node = &mut self.nodes[node_id];
        if node.outputs.iter().any(|output| output.name == name) {
            bail!("Output parameter {name} already exists for node {node_id:?}");
        } else {
            self.nodes[node_id].outputs.push(Output { name, data_type });
        }
        Ok(())
    }

    /// Registers a connection so that the `dst_param` input of `dst_node` is
    /// fulfilled by the `src_param` output of `src_node`.
    pub fn add_connection(
        &mut self,
        src_node: BjkNodeId,
        src_param: &str,
        dst_node: BjkNodeId,
        dst_param: &str,
    ) -> Result<()> {
        let src_data_type = self.nodes[src_node]
            .outputs
            .iter()
            .find(|output| output.name == src_param)
            .map(|output| output.data_type)
            .ok_or_else(|| {
                anyhow!("Input parameter named {dst_param} does not exist for node {dst_node:?}")
            })?;

        if let Some(input) = self.nodes[dst_node]
            .inputs
            .iter_mut()
            .find(|input| input.name == dst_param)
        {
            if input.data_type != src_data_type {
                bail!(
                    "Incompatible types. Input is {:?}, but its corresponding output is {:?}",
                    input.data_type,
                    src_data_type
                );
            }

            input.kind = DependencyKind::Connection {
                node: src_node,
                param_name: src_param.into(),
            }
        } else {
            bail!("Input parameter named {dst_param} does not exist for node {dst_node:?}");
        }
        Ok(())
    }
}
