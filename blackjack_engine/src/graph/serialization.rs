use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::Path,
    ptr::write_bytes,
};

use anyhow::{anyhow, bail};
use itertools::Itertools;
use ron::ser::PrettyConfig;
use serde::{de::Visitor, Deserialize, Serialize};
use slotmap::SecondaryMap;

use crate::graph_interpreter::{ExternalParameter, ExternalParameterValues};

use super::{
    BjkGraph, BjkNode, BjkNodeId, BlackjackParameter, BlackjackValue, DataType, InputParameter,
    Output,
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

    pub fn to_writer(&self, mut w: impl Write) {
        // Serde made it very inconvenient to deserialize the version field
        // before attempting to deserialize the whole RON file. A pragmatic
        // solution was to (ab)use RON's comment support to encode version
        // metadata as a comment on the first line.
        //
        // This "comment" is just as part of the BLJ file format as the
        // subsequent RON data, so if a user tampers with it they will corrupt
        // the file, same as if they arbitrarily removed parts of the RON data.
        writeln!(
            w,
            "// BLACKJACK_VERSION_HEADER {} {} {}",
            self.major, self.minor, self.patch
        );
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
pub struct SerializedNodePositions {
    pub node_positions: Vec<glam::Vec2>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SerializedParamLocation {
    pub node_idx: usize,
    pub param_name: String,
}

#[derive(Serialize, Deserialize)]
pub enum SerializedParameterConfig {
    // Scalar
    ScalarDefault(f32),
    ScalarSoftMin(f32),
    ScalarSoftMax(f32),
    ScalarMin(f32),
    ScalarMax(f32),
    ScalarNumDecimals(f32),

    // Vector
    VectorDefault(glam::Vec3),

    // Selection
    SelectionDefault(String),

    // String (general)
    StringDefault(String),

    // Enum
    StringEnumValues(Vec<String>),
    StringEnumDefaultSelection(u32),

    // FilePath
    StringFilePathMode(String),

    // Basic string
    StringMultiline(bool),

    // LuaString
    StringCode(bool),
}

#[derive(Serialize, Deserialize)]
pub struct SerializedParameterConfigs {
    pub param_values: HashMap<SerializedParamLocation, Vec<SerializedParameterConfig>>,
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
    pub node_positions: Option<SerializedNodePositions>,
    pub external_parameters: Option<SerializedExternalParameters>,
    pub parameter_configs: Option<SerializedParameterConfigs>,
}

/// Maps slotmap ids to serialized indices.
type IdToIdx = SecondaryMap<BjkNodeId, usize>;

/// Maps serialized indices to node ids
type IdxToId = Vec<BjkNodeId>;

struct IdMappings {
    id_to_idx: IdToIdx,
    idx_to_id: IdxToId,
}

/// This struct represents the runtime data that can be written to, or loaded
/// from a serialized file.
pub struct RuntimeData {
    pub graph: BjkGraph,
    pub external_parameters: Option<ExternalParameterValues>,
    pub positions: Option<SecondaryMap<BjkNodeId, glam::Vec2>>,
    pub parameters: Option<Vec<(BjkNodeId, String, BlackjackParameter)>>,
}

// ===========================================
// ==== SERIALIZATION FROM RUNTIME VALUES ====
// ===========================================

impl SerializedBjkGraph {
    pub fn write_to_file(path: impl AsRef<Path>, runtime_data: RuntimeData) -> anyhow::Result<()> {
        let version = SerializationVersion::latest();
        let data = Self::from_runtime(runtime_data);
        let mut writer = BufWriter::new(std::fs::File::create(path)?);
        version.to_writer(&mut writer);
        ron::ser::to_writer_pretty(&mut writer, &data, PrettyConfig::default())?;
        Ok(())
    }

    pub fn from_runtime(runtime_data: RuntimeData) -> Self {
        let RuntimeData {
            graph,
            external_parameters,
            positions,
            parameters,
        } = runtime_data;
        let BjkGraph { nodes } = graph;

        let mappings = IdMappings {
            id_to_idx: nodes.keys().zip(0..).collect(),
            idx_to_id: nodes.keys().collect(),
        };

        let mut serialized_nodes = vec![];
        for (node_id, node) in nodes {
            serialized_nodes.push(SerializedBjkNode::from_runtime_data(
                node_id, &node, &mappings,
            ));
        }

        Self {
            nodes: serialized_nodes,
            node_positions: positions.map(|p| SerializedNodePositions::from_runtime(p, &mappings)),
            external_parameters: external_parameters
                .map(|e| SerializedExternalParameters::from_runtime(e, &mappings)),
            parameter_configs: parameters
                .map(|p| SerializedParameterConfigs::from_runtime(p, &mappings)),
        }
    }
}

impl SerializedNodePositions {
    fn from_runtime(positions: SecondaryMap<BjkNodeId, glam::Vec2>, mapping: &IdMappings) -> Self {
        SerializedNodePositions {
            node_positions: mapping
                .idx_to_id
                .iter()
                .enumerate()
                .map(|(idx, id)| positions.get(*id).copied().unwrap_or(glam::Vec2::ZERO))
                .collect::<Vec<_>>(),
        }
    }
}

impl SerializedParameterConfigs {
    fn from_runtime(
        parameters: Vec<(BjkNodeId, String, BlackjackParameter)>,
        mapping: &IdMappings,
    ) -> Self {
        SerializedParameterConfigs {
            param_values: parameters
                .into_iter()
                .filter_map(|(node_id, param_name, param)| {
                    if let Some(idx) = mapping.id_to_idx.get(node_id) {
                        Some((
                            SerializedParamLocation {
                                node_idx: *idx,
                                param_name,
                            },
                            { SerializedParameterConfig::from_runtime_data(param.clone()) },
                        ))
                    } else {
                        println!(
                            "WARNING: Found parameter config for non-existing node {node_id:?}"
                        );
                        None
                    }
                })
                .collect(),
        }
    }
}

impl SerializedExternalParameters {
    fn from_runtime(
        external_param_values: ExternalParameterValues,
        mapping: &IdMappings,
    ) -> SerializedExternalParameters {
        SerializedExternalParameters {
            param_values: external_param_values
                .0
                .iter()
                .filter_map(|(loc, value)| {
                    if let Some(val) = SerializedBlackjackValue::from_runtime(value.clone()) {
                        let ExternalParameter {
                            node_id,
                            param_name,
                        } = loc;
                        Some((
                            SerializedParamLocation {
                                node_idx: mapping.id_to_idx[*node_id],
                                param_name: param_name.clone(),
                            },
                            val,
                        ))
                    } else {
                        None
                    }
                })
                .collect::<HashMap<_, _>>(),
        }
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

impl SerializedParameterConfig {
    pub fn from_runtime_data(param: BlackjackParameter) -> Vec<Self> {
        let mut configs = Vec::<Self>::new();

        macro_rules! add {
            ($p:path, $e:expr) => {
                configs.push($p($e))
            };
        }

        macro_rules! add_option {
            ($p:path, $i:ident) => {
                if let Some(inner) = $i {
                    configs.push($p(inner));
                }
            };
        }

        use SerializedParameterConfig::*;
        match param.config {
            super::InputValueConfig::Vector { default } => {
                add!(VectorDefault, default);
            }
            super::InputValueConfig::Scalar {
                default,
                min,
                max,
                soft_min,
                soft_max,
                num_decimals,
            } => {
                add!(ScalarDefault, default);
                add_option!(ScalarMin, min);
                add_option!(ScalarMax, min);
                add_option!(ScalarSoftMin, min);
                add_option!(ScalarSoftMax, min);
                add_option!(ScalarNumDecimals, min);
            }
            super::InputValueConfig::Selection { default_selection } => {
                add!(SelectionDefault, default_selection.unparse());
            }
            super::InputValueConfig::Enum {
                values,
                default_selection,
            } => {
                add!(StringEnumValues, values);
                add_option!(StringEnumDefaultSelection, default_selection);
            }
            super::InputValueConfig::FilePath {
                default_path,
                file_path_mode,
            } => {
                add!(
                    StringFilePathMode,
                    match file_path_mode {
                        crate::graph::FilePathMode::Open => "Open".into(),
                        crate::graph::FilePathMode::Save => "Save".into(),
                    }
                );
                add_option!(StringDefault, default_path);
            }
            super::InputValueConfig::String {
                multiline,
                default_text,
            } => {
                if multiline {
                    add!(StringMultiline, true);
                }
                add!(StringDefault, default_text);
            }
            super::InputValueConfig::LuaString {} => {
                add!(StringCode, true);
            }
            super::InputValueConfig::None => {}
        }

        configs
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

// ====================================================
// ==== RUNTIME DATA GENERATION FROM STORED VALUES ====
// ====================================================

impl SerializedBjkGraph {
    pub fn to_runtime_data(self) -> RuntimeData {
        let graph = BjkGraph {
            nodes: todo!()
        };

        // WIP
        RuntimeData {
            graph: (),
            external_parameters: (),
            positions: (),
            parameters: (),
        }
    }
}

#[cfg(test)]
mod tests {
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
        version.to_writer(writer.get_ref());
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
