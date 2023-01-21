// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::cell::{Ref, RefCell};
use std::collections::BTreeMap;
use std::ops::Deref;
use std::rc::Rc;

use crate::prelude::*;
use crate::{lua_engine::lua_stdlib::LVec3, mesh::halfedge::selection::SelectionExpression};
use anyhow::{anyhow, Result};
use mlua::{FromLua, Table, ToLua};
use slotmap::SlotMap;

/// The core `bjk` file format
pub mod serialization;

pub struct LuaExpression(pub String);

/// A node has inputs (dependencies) that need to be met. A dependency can be
/// met in three different ways.
#[derive(Debug)]
pub enum DependencyKind {
    /// Taking the value of an external parameter, from the inputs to the graph
    /// function itself.
    ///
    /// When promoted, the connection stores the parameter name that will be
    /// shown to the user of the graph.
    External { promoted: Option<String> },
    /// Taking the value from another node's outputs.
    Connection { node: BjkNodeId, param_name: String },
}

/// The data types available for graph parameters
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DataType {
    Vector,
    Scalar,
    Selection,
    Mesh,
    String,
    HeightMap,
}

impl DataType {
    /// Returns whether this datatype can be rendered into a final artifact
    pub fn can_be_enabled(&self) -> bool {
        match self {
            DataType::Mesh | DataType::HeightMap => true,
            DataType::Vector | DataType::Scalar | DataType::Selection | DataType::String => false,
        }
    }

    /// Returns whether the given value is valid for this data type
    pub fn is_valid_value(&self, value: &BlackjackValue) -> bool {
        match self {
            DataType::Vector => matches!(value, BlackjackValue::Vector(_)),
            DataType::Scalar => matches!(value, BlackjackValue::Scalar(_)),
            DataType::Selection => matches!(value, BlackjackValue::Selection(_, _)),
            DataType::String => matches!(value, BlackjackValue::String(_)),
            DataType::Mesh => matches!(value, BlackjackValue::None),
            DataType::HeightMap => matches!(value, BlackjackValue::None),
        }
    }
}

#[derive(Debug, Clone)]
pub enum BlackjackValue {
    Vector(glam::Vec3),
    Scalar(f32),
    String(String),
    Selection(String, Option<SelectionExpression>),
    None,
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

impl<'lua> FromLua<'lua> for BlackjackValue {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        let type_name = lua_value.type_name();
        match lua_value {
            mlua::Value::Nil => return Ok(BlackjackValue::None),
            mlua::Value::Integer(i) => return Ok(BlackjackValue::Scalar(i as f32)),
            mlua::Value::Number(n) => return Ok(BlackjackValue::Scalar(n as f32)),
            mlua::Value::Vector(x, y, z) => {
                return Ok(BlackjackValue::Vector(glam::Vec3::new(x, y, z)))
            }
            mlua::Value::String(s) => return Ok(BlackjackValue::String(s.to_str()?.into())),
            mlua::Value::UserData(u) => {
                if u.is::<SelectionExpression>() {
                    let sel = u.borrow::<SelectionExpression>()?.clone();
                    return Ok(BlackjackValue::Selection(sel.unparse(), Some(sel)));
                }
            }
            _ => {}
        }
        Err(mlua::Error::FromLuaConversionError {
            from: type_name,
            to: "BlackjackValue",
            message: Some("Could not convert to blackjack value".into()),
        })
    }
}

/// An input parameter in the graph. Inputs represent data dependencies that
/// need to be met before executing a node.
#[derive(Debug)]
pub struct InputParameter {
    pub name: String,
    pub data_type: DataType,
    pub kind: DependencyKind,
}

/// An output parameter. Outputs are pieces of data produced by a node, which
/// can be used to feed into another nodes as inputs.
#[derive(Debug)]
pub struct Output {
    pub name: String,
    pub data_type: DataType,
}

/// A node in the blackjack graph
#[derive(Debug)]
pub struct BjkNode {
    pub op_name: String,
    /// When this node is the target of a graph, this stores the name of the
    /// output parameter that should be displayed (typically, a mesh).
    pub return_value: Option<String>,
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
    /// When the graph is run, this is the node that will be executed by default.
    pub default_node: Option<BjkNodeId>,
}

/// Represents a fragment of a `BjkGraph`. Snippets can be taken out of a graph
/// and applied to another graph. They are the backing data structure to
/// implement copy & paste.
///
/// NOTE: At this time, the `BjkSnippet` doesn't have substantial differences
/// with `BjkGraph`, but the distinction between the two types is there to
/// accomodate for future differences between a *portion* of a graph and a full,
/// executable graph.
#[derive(Default)]
pub struct BjkSnippet {
    pub nodes: SlotMap<BjkNodeId, BjkNode>,
}

/// Specifies the ways in which the file picker dialog for an
/// `InputValueConfig::FilePath` can work.
#[derive(Debug, Copy, Clone)]
pub enum FilePathMode {
    /// The file picker will only let the user select an existing file
    Open,
    /// The file picker will let the user choose a new file or an existing one,
    /// with an overwrite warning.
    Save,
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
#[derive(Debug, Clone)]
pub enum InputValueConfig {
    Vector {
        default: glam::Vec3,
    },
    Scalar {
        default: f32,
        min: Option<f32>,
        max: Option<f32>,
        soft_min: Option<f32>,
        soft_max: Option<f32>,
        num_decimals: Option<u32>,
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
        file_path_mode: FilePathMode,
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

impl DataType {
    /// Returns the default value for this data type. This does not take
    /// parameter configuration into account. For that, use
    /// `InputDefinition::default_value`
    pub fn default_value(&self) -> BlackjackValue {
        match self {
            DataType::Vector => BlackjackValue::Vector(Vec3::default()),
            DataType::Scalar => BlackjackValue::Scalar(0.0),
            DataType::Selection => {
                BlackjackValue::Selection("".into(), Some(SelectionExpression::None))
            }
            DataType::String => BlackjackValue::String("".into()),
            DataType::Mesh => BlackjackValue::None,
            DataType::HeightMap => BlackjackValue::None,
        }
    }
}

impl InputDefinition {
    pub fn default_value(&self) -> BlackjackValue {
        let default_string = || BlackjackValue::String("".into());

        match (&self.data_type, &self.config) {
            (DataType::Vector, InputValueConfig::Vector { default }) => {
                BlackjackValue::Vector(*default)
            }
            (DataType::Scalar, InputValueConfig::Scalar { default, .. }) => {
                BlackjackValue::Scalar(*default)
            }
            (DataType::Selection, InputValueConfig::Selection { default_selection }) => {
                BlackjackValue::Selection(
                    default_selection.unparse(),
                    Some(default_selection.clone()),
                )
            }
            (DataType::Mesh, InputValueConfig::None) => BlackjackValue::None,
            (
                DataType::String,
                InputValueConfig::Enum {
                    values,
                    default_selection,
                },
            ) => {
                if let Some(default) = default_selection {
                    values
                        .get(*default as usize)
                        .cloned()
                        .map(BlackjackValue::String)
                        .unwrap_or_else(default_string)
                } else {
                    default_string()
                }
            }
            (DataType::String, InputValueConfig::FilePath { default_path, .. }) => default_path
                .as_ref()
                .cloned()
                .map(BlackjackValue::String)
                .unwrap_or_else(default_string),
            (DataType::String, InputValueConfig::String { default_text, .. }) => {
                BlackjackValue::String(default_text.clone())
            }
            (DataType::String, InputValueConfig::LuaString {}) => default_string(),
            (DataType::HeightMap, InputValueConfig::None) => BlackjackValue::None,

            // Fallback: When config is not valud, return some valid value
            (data_type, _) => data_type.default_value(),
        }
    }
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
    /// This node has an available interactive gizmo.
    pub has_gizmo: bool,
}

#[derive(Default)]
pub struct NodeDefinitionsInner(BTreeMap<String, NodeDefinition>);

/// A collection of node definitions. This struct is the Rust counterpart to the
/// node library in Lua.
///
/// This struct acts like a pointer with multiple ownership and interior
/// mutability, allowing multiple locations in the codebase to store and receive
/// updates to the node definitions when hot-reloading detects changes.
#[derive(Default)]
pub struct NodeDefinitions {
    pub inner: Rc<RefCell<NodeDefinitionsInner>>,
}

impl NodeDefinitions {
    pub fn new(inner: NodeDefinitionsInner) -> Self {
        Self {
            inner: Rc::new(RefCell::new(inner)),
        }
    }
    pub fn share(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
    pub fn node_names(&self) -> Vec<String> {
        self.inner.borrow().0.keys().cloned().collect()
    }
    pub fn node_def(&self, op_name: &str) -> Option<impl Deref<Target = NodeDefinition> + '_> {
        let guard = self.inner.borrow();
        if guard.0.contains_key(op_name) {
            Some(Ref::map(guard, |x| x.0.get(op_name).unwrap()))
        } else {
            None
        }
    }
    pub fn update(&self, new_data: NodeDefinitionsInner) {
        *self.inner.borrow_mut() = new_data;
    }
}

/// Given a string representing an input definition type (taken from a Lua
/// file), returns the data type for that parameter.
fn data_type_from_str(s: &str) -> Result<DataType> {
    match s {
        "vec3" => Ok(DataType::Vector),
        "scalar" => Ok(DataType::Scalar),
        "selection" => Ok(DataType::Selection),
        "mesh" => Ok(DataType::Mesh),
        "heightmap" => Ok(DataType::HeightMap),
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
                min: table.get::<_, Option<f32>>("min")?,
                max: table.get::<_, Option<f32>>("max")?,
                soft_min: table.get::<_, Option<f32>>("soft_min")?,
                soft_max: table.get::<_, Option<f32>>("soft_max")?,
                num_decimals: table.get::<_, Option<u32>>("num_decimals")?,
            },
            DataType::Selection => InputValueConfig::Selection {
                default_selection: SelectionExpression::None,
            },
            DataType::Mesh => InputValueConfig::None,
            DataType::HeightMap => InputValueConfig::None,
            DataType::String if type_str == "enum" => InputValueConfig::Enum {
                values: table
                    .get::<_, Table>("values")?
                    .sequence_values::<String>()
                    .collect::<Result<Vec<_>, _>>()?,
                default_selection: table.get::<_, Option<u32>>("selected")?,
            },
            DataType::String if type_str == "file" => {
                let mode = table.get::<_, String>("mode")?;
                InputValueConfig::FilePath {
                    default_path: None,
                    file_path_mode: if mode == "open" {
                        FilePathMode::Open
                    } else if mode == "save" {
                        FilePathMode::Save
                    } else {
                        bail!("Undefined mode {mode}")
                    },
                }
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
            has_gizmo: table.get::<_, mlua::Value>("gizmos")? != mlua::Value::Nil,
        })
    }

    /// Loads a group of [`NodeDefinitions`] from a Lua table
    pub fn load_nodes_from_table(table: Table) -> Result<NodeDefinitionsInner> {
        Ok(NodeDefinitionsInner(
            table
                .pairs::<String, Table>()
                .map(|pair| {
                    let (k, v) = pair?;
                    Ok((k.clone(), NodeDefinition::from_lua(k, v)?))
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl BjkGraph {
    // Constructs an empty graph
    pub fn new() -> Self {
        Self {
            nodes: Default::default(),
            default_node: None,
        }
    }
    /// Adds a new empty node to the graph
    pub fn add_node(&mut self, op_name: impl ToString, return_value: Option<String>) -> BjkNodeId {
        self.nodes.insert(BjkNode {
            op_name: op_name.to_string(),
            return_value,
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
        promoted: Option<String>,
    ) -> Result<()> {
        let name = name.to_string();
        let node = &mut self.nodes[node_id];
        if node.inputs.iter().any(|input| input.name == name) {
            bail!("Input parameter {name} already exists for node {node_id:?}");
        } else {
            self.nodes[node_id].inputs.push(InputParameter {
                name,
                data_type,
                kind: DependencyKind::External { promoted },
            });
        }
        Ok(())
    }

    /// Registers a new input for `node_id`
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
