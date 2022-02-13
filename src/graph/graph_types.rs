use glam::Vec3;
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;
use std::collections::HashMap;

use crate::prelude::SVec;

mod id_types;
pub use id_types::*;

mod graph_impls;
mod index_impls;

pub mod node_types;
mod param_ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
pub enum InputParamMetadata {
    MinMaxScalar { min: f32, max: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputParamValue {
    Vector(Vec3),
    Scalar(f32),
    Selection {
        text: String,
        selection: Option<Vec<u32>>,
    },
    /// Used for parameters that can't have a value because they only accept
    /// connections.
    None,
    Enum {
        values: Vec<String>,
        selection: Option<u32>,
    },
    NewFile {
        path: Option<std::path::PathBuf>,
    },
}

/// There are three kinds of input params
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InputParamKind {
    /// No constant value can be set. Only incoming connections can produce it
    ConnectionOnly,
    /// Only a constant value can be set. No incoming connections accepted.
    ConstantOnly,
    /// Both incoming connections and constants are accepted. Connections take
    /// precedence over the constant values.
    ConnectionOrConstant,
}

fn shown_inline_default() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputParam {
    id: InputId,
    /// The data type of this node. Used to determine incoming connections. This
    /// should always match the type of the InputParamValue, but the property is
    /// not actually enforced.
    typ: DataType,
    /// The constant value stored in this parameter.
    value: InputParamValue,
    /// A list of metadata fields, specifying things like bounds or limits.
    /// Metadata values that don't make sense for a type are ignored.
    metadata: SVec<InputParamMetadata>,
    /// The input kind. See [InputParamKind]
    kind: InputParamKind,
    /// Back-reference to the node containing this parameter.
    node: NodeId,
    /// When true, the node is shown inline inside the node graph.
    #[serde(default = "shown_inline_default")]
    pub shown_inline: bool,
}

impl InputParam {
    pub fn value(&self) -> InputParamValue {
        self.value.clone()
    }

    pub fn kind(&self) -> InputParamKind {
        self.kind
    }

    pub fn node(&self) -> NodeId {
        self.node
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputParam {
    id: OutputId,
    /// Back-reference to the node containing this parameter.
    node: NodeId,
    typ: DataType,
}

impl OutputParam {
    pub fn node(&self) -> NodeId {
        self.node
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub label: String,
    pub op_name: String,
    pub inputs: Vec<(String, InputId)>,
    pub outputs: Vec<(String, OutputId)>,
    /// Executable nodes will run some code when their "Run" button is clicked
    pub is_executable: bool,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    nodes: SlotMap<NodeId, Node>,
    inputs: SlotMap<InputId, InputParam>,
    outputs: SlotMap<OutputId, OutputParam>,
    // Connects the input of a node, to the output of its predecessor that
    // produces it
    connections: HashMap<InputId, OutputId>,
}

pub enum InputDescriptor {
    Vector { default: Vec3 },
    Mesh,
    Selection,
    Scalar { default: f32, min: f32, max: f32 },
    Enum { default: Option<u32>, values: Vec<String> },
    NewFile,
}

pub struct OutputDescriptor(DataType);

pub struct NodeDescriptor {
    pub op_name: String,
    pub label: String,
    pub inputs: Vec<(String, InputDescriptor)>,
    pub outputs: Vec<(String, OutputDescriptor)>,
    pub is_executable: bool,
}
