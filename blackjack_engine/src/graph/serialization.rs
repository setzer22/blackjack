use serde::{Deserialize, Serialize};
use slotmap::SecondaryMap;

use super::{BjkGraph, BjkNode, BjkNodeId, DataType, InputParameter, Output};

#[derive(Serialize, Deserialize)]
pub struct SerializationVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SerializationVersion {
    pub fn latest() -> Self {
        Self {
            major: 0,
            minor: 1,
            patch: 0,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum SerializedDependencyKind {
    Computed(String),
    External,
    Conection { node_idx: usize, param_name: String },
}

#[derive(Serialize, Deserialize)]
pub struct SerializedInput {
    pub name: String,
    pub data_type: String,
    pub kind: SerializedDependencyKind,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedOutput {
    pub name: String,
    pub data_type: String,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedBjkNode {
    pub op_name: String,
    pub return_value: Option<String>,
    pub inputs: Vec<SerializedInput>,
    pub outputs: Vec<SerializedOutput>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedNodePositions {
    node_positions: Vec<glam::Vec2>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedExternalParameters {

}

#[derive(Serialize, Deserialize)]
pub struct SerializedBjkGraph {
    pub version: SerializationVersion,
    pub nodes: Vec<SerializedBjkNode>,
    pub node_positions: Option<SerializedNodePositions>,
    pub external_parameters: SerializedExternalParameters,
}

/// Maps slotmap ids to serialized indices.
type IdToIdx = SecondaryMap<BjkNodeId, usize>;

/// Maps serialized indices to slotmap ids.
type IdxToId = Vec<BjkNodeId>;

struct IdMappings {
    id_to_idx: IdToIdx,
    idx_to_id: IdxToId,
}

impl SerializedBjkGraph {
    pub fn from_runtime_data(graph: &BjkGraph) -> Self {
        let BjkGraph { nodes } = graph;

        let mappings = IdMappings {
            id_to_idx: nodes.keys().zip(0..).collect(),
            idx_to_id: nodes.keys().collect(),
        };

        let mut serialized_nodes = vec![];
        for (node_id, node) in nodes {
            serialized_nodes.push(SerializedBjkNode::from_runtime_data(
                node_id, node, &mappings,
            ));
        }

        Self {
            version: SerializationVersion::latest(),
            nodes: serialized_nodes,
            node_positions: None,
            // WIP: Serialize external parameters as well.

        }
    }
}

impl SerializedBjkNode {
    fn from_runtime_data(node_id: BjkNodeId, node: &BjkNode, mappings: &IdMappings) -> Self {
        let BjkNode {
            op_name,
            return_value,
            inputs,
            outputs,
        } = node;

        let inputs = inputs
            .iter()
            .map(|input| SerializedInput::from_runtime_data(input, mappings))
            .collect();
        let outputs = outputs
            .iter()
            .map(|output| SerializedOutput::from_runtime_data(output, mappings))
            .collect();

        Self {
            op_name: op_name.clone(),
            return_value: return_value.clone(),
            inputs,
            outputs,
        }
    }
}

fn serialize_data_type(data_type: DataType) -> String {
    match data_type {
        super::DataType::Vector => "BLJ_VECTOR",
        super::DataType::Scalar => "BLJ_SCALAR",
        super::DataType::Selection => "BLJ_SELECTION",
        super::DataType::Mesh => "BLJ_MESH",
        super::DataType::String => "BLJ_STRING",
        super::DataType::HeightMap => "BLJ_HEIGHTMAP",
    }
    .to_owned()
}

impl SerializedInput {
    fn from_runtime_data(input: &super::InputParameter, mappings: &IdMappings) -> Self {
        let InputParameter {
            name,
            data_type,
            kind,
        } = input;

        let dependency_kind = SerializedDependencyKind::from_runtime_data(kind, mappings);

        Self {
            name: name.clone(),
            data_type: serialize_data_type(*data_type),
            kind: dependency_kind,
        }
    }
}

impl SerializedOutput {
    fn from_runtime_data(output: &super::Output, mappings: &IdMappings) -> Self {
        let Output { name, data_type } = output;
        Self {
            name: name.clone(),
            data_type: serialize_data_type(*data_type),
        }
    }
}
impl SerializedDependencyKind {
    fn from_runtime_data(kind: &super::DependencyKind, mappings: &IdMappings) -> Self {
        match kind {
            super::DependencyKind::Computed(lua_expr) => Self::Computed(lua_expr.0.clone()),
            super::DependencyKind::External => Self::External,
            super::DependencyKind::Connection { node, param_name } => Self::Conection {
                node_idx: mappings.id_to_idx[*node] as usize,
                param_name: param_name.clone(),
            },
        }
    }
}
