// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

use anyhow::{anyhow, bail, Result};
use itertools::Itertools;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use slotmap::{SecondaryMap, SlotMap};

use crate::{
    graph_interpreter::{ExternalParameter, ExternalParameterValues},
    prelude::selection::SelectionExpression,
};

use super::{
    BjkGraph, BjkNode, BjkNodeId, BjkSnippet, BlackjackValue, DataType, DependencyKind,
    InputParameter, Output,
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
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

    pub fn to_writer(&self, mut w: impl Write) -> Result<()> {
        // Serde made it very inconvenient to deserialize the version field
        // before attempting to deserialize the whole RON file. A pragmatic
        // solution was to (ab)use RON's comment support to encode version
        // metadata as a comment on the first line.
        //
        // This "comment" is just as part of the BJK file format as the
        // subsequent RON data, so if a user tampers with it they will corrupt
        // the file, same as if they arbitrarily removed parts of the RON data.
        writeln!(
            w,
            "// BLACKJACK_VERSION_HEADER {} {} {}",
            self.major, self.minor, self.patch
        )?;
        Ok(())
    }

    pub fn from_reader(mut r: impl BufRead) -> Result<Self, anyhow::Error> {
        let mut header_line = String::new();
        r.read_line(&mut header_line)?;

        let header = header_line.trim_end_matches('\n').split(' ').collect_vec();
        match header.as_slice() {
            &[_, header_str, major_str, minor_str, patch_str] => {
                if header_str != "BLACKJACK_VERSION_HEADER" {
                    bail!("Blackjack files should start with a version header.");
                }
                Ok(Self {
                    major: major_str.parse().map_err(|err| {
                        anyhow!("Could not parse version major '{major_str}'. {err}")
                    })?,
                    minor: minor_str.parse().map_err(|err| {
                        anyhow!("Could not parse version minor '{minor_str}'. {err}")
                    })?,
                    patch: patch_str.parse().map_err(|err| {
                        anyhow!("Could not parse version patch '{patch_str}'. {err}")
                    })?,
                })
            }
            _ => {
                bail!("Invalid blackjack version header.")
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum SerializedDependencyKind {
    External { promoted: Option<String> },
    Conection { node_idx: usize, param_name: String },
}

#[derive(Serialize, Deserialize)]
pub struct SerializedInput {
    pub name: String,
    pub data_type: String,
    pub kind: SerializedDependencyKind,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
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
pub struct SerializedUiData {
    pub node_positions: Vec<glam::Vec2>,
    pub node_order: Vec<usize>,
    pub pan: glam::Vec2,
    pub zoom: f32,
    #[serde(default)]
    pub locked_gizmo_nodes: Vec<usize>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SerializedParamLocation {
    pub node_idx: usize,
    pub param_name: String,
}

#[derive(Serialize, Deserialize)]
pub enum SerializedBlackjackValue {
    Vector(glam::Vec3),
    Scalar(f32),
    String(String),
    Selection(String),
}

#[derive(Serialize, Deserialize)]
pub struct SerializedExternalParameters {
    pub param_values: HashMap<SerializedParamLocation, SerializedBlackjackValue>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedBjkGraph {
    pub nodes: Vec<SerializedBjkNode>,
    pub default_node: Option<usize>,
    pub ui_data: Option<SerializedUiData>,
    pub external_parameters: Option<SerializedExternalParameters>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SerializedBjkSnippet {
    pub nodes: Vec<SerializedBjkNode>,
    pub node_relative_positions: Option<Vec<glam::Vec2>>,
    pub external_parameters: Option<SerializedExternalParameters>,
}

/// Maps slotmap ids to serialized indices.
type IdToIdx = SecondaryMap<BjkNodeId, usize>;

/// Maps serialized indices to node ids
type IdxToId = Vec<BjkNodeId>;

#[derive(Default)]
pub struct IdMappings {
    pub id_to_idx: IdToIdx,
    pub idx_to_id: IdxToId,
}

impl IdMappings {
    pub fn get_id(&self, idx: usize) -> Result<BjkNodeId> {
        self.idx_to_id
            .get(idx)
            .ok_or_else(|| anyhow!("Invalid stored index {idx}"))
            .copied()
    }

    pub fn get_idx(&self, id: BjkNodeId) -> Result<usize> {
        self.id_to_idx
            .get(id)
            .ok_or_else(|| anyhow!("Invalid node id {id:?}"))
            .copied()
    }
}

/// This struct represents the runtime data that can be written to, or loaded
/// from a serialized file.
pub struct RuntimeData {
    pub graph: BjkGraph,
    pub external_parameters: Option<ExternalParameterValues>,
}

/// This struct represents the runtime data that can be copied to, or pasted
/// from the user's clipboard.
pub struct SnippetRuntimeData {
    pub snippet: BjkSnippet,
    pub external_parameters: Option<ExternalParameterValues>,
}

// ===========================================
// ==== SERIALIZATION FROM RUNTIME VALUES ====
// ===========================================

impl IdMappings {
    pub fn from_nodes(nodes: &SlotMap<BjkNodeId, BjkNode>) -> Self {
        Self {
            id_to_idx: nodes.keys().zip(0..).collect(),
            idx_to_id: nodes.keys().collect(),
        }
    }
}

impl SerializedBjkGraph {
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let version = SerializationVersion::latest();
        let mut writer = BufWriter::new(std::fs::File::create(path)?);
        version.to_writer(&mut writer)?;
        ron::ser::to_writer_pretty(&mut writer, &self, PrettyConfig::default())?;
        Ok(())
    }

    pub fn from_runtime(runtime_data: RuntimeData) -> Result<(Self, IdMappings)> {
        let RuntimeData {
            graph,
            external_parameters,
        } = runtime_data;

        let mappings = IdMappings::from_nodes(&graph.nodes);

        let BjkGraph {
            nodes,
            default_node,
        } = graph;

        let mut serialized_nodes = vec![];
        for (_node_id, node) in nodes {
            serialized_nodes.push(SerializedBjkNode::from_runtime_data(&node, &mappings)?);
        }

        Ok((
            Self {
                nodes: serialized_nodes,
                default_node: default_node.and_then(|x| mappings.get_idx(x).ok()),
                external_parameters: if let Some(e) = external_parameters {
                    Some(SerializedExternalParameters::from_runtime(e, &mappings)?)
                } else {
                    None
                },
                ui_data: None,
            },
            mappings,
        ))
    }

    pub fn set_ui_data(&mut self, ui_data: SerializedUiData) {
        self.ui_data = Some(ui_data);
    }
}

impl SerializedBjkSnippet {
    pub fn into_string(&self) -> Result<String> {
        let mut w = BufWriter::new(Vec::<u8>::new());
        SerializationVersion::latest().to_writer(&mut w)?;
        ron::ser::to_writer_pretty(&mut w, self, PrettyConfig::default())?;
        Ok(String::from_utf8(w.into_inner()?)?)
    }

    pub fn from_runtime(
        mut graph: BjkGraph,
        mut external_parameters: ExternalParameterValues,
        node_projection: &[BjkNodeId],
    ) -> Result<(Self, IdMappings)> {
        // Remove the nodes from the graph that are not part of the projection
        graph.nodes.retain(|id, _| node_projection.contains(&id));

        // When there is a connection that crosses the projection boundary, we remove it.
        for (node_id, node) in &mut graph.nodes {
            if node_projection.contains(&node_id) {
                for input in &mut node.inputs {
                    if let DependencyKind::Connection { node, .. } = &input.kind {
                        if !node_projection.contains(node) {
                            input.kind = DependencyKind::External { promoted: None };
                        }
                    }
                }
            }
        }

        // We also remove any external parameters referencing nodes outside the projection
        external_parameters
            .0
            .retain(|param, _| node_projection.contains(&param.node_id));

        let mut serialized_nodes = vec![];
        let mappings = IdMappings::from_nodes(&graph.nodes);

        // Finally, we serialize as normal
        for (node_id, node) in &graph.nodes {
            debug_assert!(node_projection.contains(&node_id));
            serialized_nodes.push(SerializedBjkNode::from_runtime_data(node, &mappings)?);
        }

        Ok((
            Self {
                nodes: serialized_nodes,
                external_parameters: Some(SerializedExternalParameters::from_runtime(
                    external_parameters,
                    &mappings,
                )?),
                node_relative_positions: None,
            },
            mappings,
        ))
    }

    pub fn set_node_relative_positions(&mut self, node_relative_positions: Vec<glam::Vec2>) {
        self.node_relative_positions = Some(node_relative_positions)
    }
}

impl SerializedExternalParameters {
    fn from_runtime(
        external_param_values: ExternalParameterValues,
        mapping: &IdMappings,
    ) -> Result<SerializedExternalParameters> {
        let mut param_values = HashMap::new();
        for (loc, value) in external_param_values.0 {
            if let Some(val) = SerializedBlackjackValue::from_runtime(value.clone()) {
                let ExternalParameter {
                    node_id,
                    param_name,
                } = loc;
                param_values.insert(
                    SerializedParamLocation {
                        node_idx: mapping.get_idx(node_id)?,
                        param_name: param_name.clone(),
                    },
                    val,
                );
            }
        }

        Ok(SerializedExternalParameters { param_values })
    }
}

impl SerializedBlackjackValue {
    pub fn from_runtime(val: BlackjackValue) -> Option<Self> {
        match val {
            BlackjackValue::Vector(v) => Some(Self::Vector(v)),
            BlackjackValue::Scalar(s) => Some(Self::Scalar(s)),
            BlackjackValue::String(s) => Some(Self::String(s)),
            BlackjackValue::Selection(s, _) => Some(Self::Selection(s)),
            BlackjackValue::None => None,
        }
    }
}

impl SerializedBjkNode {
    fn from_runtime_data(node: &BjkNode, mappings: &IdMappings) -> Result<Self> {
        let BjkNode {
            op_name,
            return_value,
            inputs,
            outputs,
        } = node;

        let inputs = inputs
            .iter()
            .map(|input| SerializedInput::from_runtime_data(input, mappings))
            .collect::<Result<Vec<_>>>()?;
        let outputs = outputs
            .iter()
            .map(SerializedOutput::from_runtime_data)
            .collect();

        Ok(Self {
            op_name: op_name.clone(),
            return_value: return_value.clone(),
            inputs,
            outputs,
        })
    }
}

fn serialize_data_type(data_type: DataType) -> String {
    match data_type {
        super::DataType::Vector => "BJK_VECTOR",
        super::DataType::Scalar => "BJK_SCALAR",
        super::DataType::Selection => "BJK_SELECTION",
        super::DataType::Mesh => "BJK_MESH",
        super::DataType::String => "BJK_STRING",
        super::DataType::HeightMap => "BJK_HEIGHTMAP",
    }
    .to_owned()
}

impl SerializedInput {
    fn from_runtime_data(input: &super::InputParameter, mappings: &IdMappings) -> Result<Self> {
        let InputParameter {
            name,
            data_type,
            kind,
        } = input;

        let dependency_kind = SerializedDependencyKind::from_runtime_data(kind, mappings)?;

        Ok(Self {
            name: name.clone(),
            data_type: serialize_data_type(*data_type),
            kind: dependency_kind,
        })
    }
}

impl SerializedOutput {
    fn from_runtime_data(output: &super::Output) -> Self {
        let Output { name, data_type } = output;
        Self {
            name: name.clone(),
            data_type: serialize_data_type(*data_type),
        }
    }
}
impl SerializedDependencyKind {
    fn from_runtime_data(kind: &DependencyKind, mappings: &IdMappings) -> Result<Self> {
        match kind {
            DependencyKind::External { promoted } => Ok(Self::External {
                promoted: promoted.clone(),
            }),
            DependencyKind::Connection { node, param_name } => Ok(Self::Conection {
                node_idx: mappings.get_idx(*node)?,
                param_name: param_name.clone(),
            }),
        }
    }
}

// ====================================================
// ==== RUNTIME DATA GENERATION FROM STORED VALUES ====
// ====================================================

impl IdMappings {
    pub fn from_serialized_graph(
        nodes: &[SerializedBjkNode],
    ) -> Result<(Self, SlotMap<BjkNodeId, BjkNode>)> {
        let mut rt_nodes = SlotMap::<BjkNodeId, BjkNode>::with_key();
        let mut mappings = Self::default();
        for (idx, node) in nodes.iter().enumerate() {
            let node_id = rt_nodes.insert(BjkNode {
                op_name: node.op_name.clone(),
                return_value: node.return_value.clone(),
                inputs: vec![],
                outputs: vec![],
            });

            mappings.idx_to_id.push(node_id);
            mappings.id_to_idx.insert(node_id, idx);
        }
        Ok((mappings, rt_nodes))
    }
}

impl SerializedBjkGraph {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<SerializedBjkGraph> {
        let reader = BufReader::new(std::fs::File::open(path)?);
        Ok(ron::de::from_reader(reader)?)
    }

    pub fn load_from_string(s: &str) -> Result<SerializedBjkGraph> {
        Ok(ron::de::from_str(s)?)
    }

    pub fn into_runtime(self) -> Result<(RuntimeData, Option<SerializedUiData>, IdMappings)> {
        // This constructs the initial graph with empty data at each node
        let (mappings, mut rt_nodes) = IdMappings::from_serialized_graph(&self.nodes)?;

        // Then, we finish initializing the nodes once the mapping is complete
        for (node, node_id) in self.nodes.into_iter().zip(&mappings.idx_to_id) {
            node.fill_runtime(&mut rt_nodes[*node_id], &mappings)?;
        }

        Ok((
            RuntimeData {
                graph: BjkGraph {
                    nodes: rt_nodes,
                    default_node: self.default_node.and_then(|x| mappings.get_id(x).ok()),
                },
                external_parameters: if let Some(e) = self.external_parameters {
                    Some(e.into_runtime(&mappings)?)
                } else {
                    None
                },
            },
            self.ui_data,
            mappings,
        ))
    }
}

impl SerializedBjkNode {
    pub fn fill_runtime(self, rt_node: &mut BjkNode, mappings: &IdMappings) -> Result<()> {
        for input in self.inputs {
            if let Some(data_type) = deserialize_data_type(&input.data_type) {
                rt_node.inputs.push(InputParameter {
                    name: input.name,
                    data_type,
                    kind: match input.kind {
                        SerializedDependencyKind::External { promoted } => {
                            DependencyKind::External { promoted }
                        }
                        SerializedDependencyKind::Conection {
                            node_idx,
                            param_name,
                        } => DependencyKind::Connection {
                            node: mappings.idx_to_id[node_idx],
                            param_name,
                        },
                    },
                })
            } else {
                println!("[WARNING] Unkown data type: {}", &input.data_type)
            }
        }

        for output in self.outputs {
            if let Some(data_type) = deserialize_data_type(&output.data_type) {
                rt_node.outputs.push(Output {
                    name: output.name,
                    data_type,
                })
            } else {
                println!("[WARNING] Unkown data type: {}", &output.data_type)
            }
        }
        Ok(())
    }
}

impl SerializedBjkSnippet {
    pub fn load_from_string(s: &str) -> Result<SerializedBjkSnippet> {
        Ok(ron::de::from_str(s)?)
    }

    pub fn into_runtime(self) -> Result<(SnippetRuntimeData, Option<Vec<glam::Vec2>>, IdMappings)> {
        // This constructs the initial graph with empty data at each node
        let (mappings, mut rt_nodes) = IdMappings::from_serialized_graph(&self.nodes)?;

        // Then, we finish initializing the nodes once the mapping is complete
        for (node, node_id) in self.nodes.into_iter().zip(&mappings.idx_to_id) {
            node.fill_runtime(&mut rt_nodes[*node_id], &mappings)?;
        }

        Ok((
            SnippetRuntimeData {
                snippet: BjkSnippet { nodes: rt_nodes },
                external_parameters: if let Some(e) = self.external_parameters {
                    Some(e.into_runtime(&mappings)?)
                } else {
                    None
                },
            },
            self.node_relative_positions,
            mappings,
        ))
    }
}

fn deserialize_data_type(data_type_str: &str) -> Option<DataType> {
    match data_type_str {
        "BJK_VECTOR" => Some(super::DataType::Vector),
        "BJK_SCALAR" => Some(super::DataType::Scalar),
        "BJK_SELECTION" => Some(super::DataType::Selection),
        "BJK_MESH" => Some(super::DataType::Mesh),
        "BJK_STRING" => Some(super::DataType::String),
        "BJK_HEIGHTMAP" => Some(super::DataType::HeightMap),
        _ => None,
    }
    .to_owned()
}

impl SerializedExternalParameters {
    fn into_runtime(self, mappings: &IdMappings) -> Result<ExternalParameterValues> {
        Ok(ExternalParameterValues(
            self.param_values
                .into_iter()
                .map(|(param, value)| {
                    Ok((
                        ExternalParameter {
                            node_id: mappings.get_id(param.node_idx)?,
                            param_name: param.param_name,
                        },
                        match value {
                            SerializedBlackjackValue::Vector(x) => BlackjackValue::Vector(x),
                            SerializedBlackjackValue::Scalar(x) => BlackjackValue::Scalar(x),
                            SerializedBlackjackValue::String(x) => BlackjackValue::String(x),
                            SerializedBlackjackValue::Selection(x) => {
                                let expr = SelectionExpression::parse(&x).ok();
                                BlackjackValue::Selection(x, expr)
                            }
                        },
                    ))
                })
                .collect::<Result<HashMap<_, _>>>()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufReader};

    use super::*;

    /// Test reading the serialization version header, plus some data from a
    /// file, and confirms the information can be read back without loss.
    #[test]
    pub fn test_versioning() {
        let version = SerializationVersion {
            major: 4,
            minor: 2,
            patch: 0,
        };
        let data = SerializedOutput {
            name: "TEST_NAME".into(),
            data_type: "TEST_DATA".into(),
        };

        let mut writer = BufWriter::new(File::create("/tmp/test.ron").unwrap());
        version.to_writer(writer.get_ref()).unwrap();
        ron::ser::to_writer_pretty(&mut writer, &data, PrettyConfig::default()).unwrap();

        drop(writer);

        let mut reader = BufReader::new(File::open("/tmp/test.ron").unwrap());

        let new_version: SerializationVersion =
            SerializationVersion::from_reader(&mut reader).unwrap();
        let new_data: SerializedOutput = ron::de::from_reader(&mut reader).unwrap();

        assert_eq!(version, new_version);
        assert_eq!(data, new_data);
    }
}
