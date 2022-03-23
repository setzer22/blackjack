use super::*;
use egui_node_graph::{InputParamKind, NodeTemplateIter, NodeTemplateTrait};
use strum::IntoEnumIterator;

#[derive(Clone, Copy, strum_macros::EnumIter)]
pub enum GraphNodeType {
    MakeBox,
    MakeQuad,
    BevelEdges,
    ExtrudeFaces,
    ChamferVertices,
    MakeVector,
    VectorMath,
    MergeMeshes,
    ExportObj,
    MeshSubdivide,
}

impl GraphNodeType {
    pub fn all_types() -> impl Iterator<Item = GraphNodeType> {
        GraphNodeType::iter()
    }

    pub fn type_label(&self) -> &'static str {
        match self {
            GraphNodeType::MakeBox => "Box",
            GraphNodeType::MakeQuad => "Quad",
            GraphNodeType::BevelEdges => "Bevel edges",
            GraphNodeType::ExtrudeFaces => "Extrude faces",
            GraphNodeType::ChamferVertices => "Chamfer vertices",
            GraphNodeType::MakeVector => "Vector",
            GraphNodeType::VectorMath => "Vector math",
            GraphNodeType::MergeMeshes => "Merge meshes",
            GraphNodeType::ExportObj => "OBJ Export",
            GraphNodeType::MeshSubdivide => "Subdivide",
        }
    }

    /// The op_name is used by the graph compiler in graph_compiler.rs to select
    /// which PolyASM instructions to emit.
    pub fn op_name(&self) -> &'static str {
        match self {
            GraphNodeType::MakeBox => "MakeBox",
            GraphNodeType::MakeQuad => "MakeQuad",
            GraphNodeType::BevelEdges => "BevelEdges",
            GraphNodeType::ExtrudeFaces => "ExtrudeFaces",
            GraphNodeType::ChamferVertices => "ChamferVertices",
            GraphNodeType::MakeVector => "MakeVector",
            GraphNodeType::VectorMath => "VectorMath",
            GraphNodeType::MergeMeshes => "MergeMeshes",
            GraphNodeType::ExportObj => "ExportObj",
            GraphNodeType::MeshSubdivide => "MeshSubdivide",
        }
    }
}

impl NodeTemplateTrait for GraphNodeType {
    type NodeData = super::NodeData;

    type DataType = super::DataType;

    type ValueType = super::ValueType;

    fn node_finder_label(&self) -> &str {
        self.type_label()
    }

    fn node_graph_label(&self) -> String {
        self.type_label().into()
    }

    fn user_data(&self) -> Self::NodeData {
        Self::NodeData {
            op_name: self.op_name().into(),
            // TODO: Change this when more nodes are executable
            is_executable: matches!(self, GraphNodeType::ExportObj),
            returns: todo!(),
        }
    }

    fn build_node(
        &self,
        graph: &mut egui_node_graph::Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        node_id: egui_node_graph::NodeId,
    ) {
        macro_rules! input {
            (Vector $name:expr, $value:expr) => {
                input!(~ $name, DataType::Vector, ValueType::Vector($value),
                       InputParamKind::ConnectionOrConstant)
            };
            (Scalar $name:expr, $value:expr) => {
                input!(Scalar $name, $value, -100.0, 100.0)
            };
            (Scalar $name:expr, $value:expr, $min:expr, $max:expr) => {
                input!(~ $name, DataType::Scalar,
                       ValueType::Scalar { value: $value, min: $min, max: $max },
                       InputParamKind::ConnectionOrConstant)
            };
            (Mesh $name:expr) => {
                input!(~ $name, DataType::Mesh,
                       ValueType::None, InputParamKind::ConnectionOnly)
            };
            (Selection $name:expr) => {
                input!(~ $name, DataType::Selection,
                       ValueType::Selection { text: "".into(), selection: Some(SelectionExpression::None) },
                       InputParamKind::ConstantOnly)
            };
            (Enum $name:expr, [$($values:expr),*]) => {
                input!(~ $name, DataType::Enum,
                       ValueType::Enum { values: vec![$($values.into()),*], selected: None },
                       InputParamKind::ConstantOnly)
            };
            (Enum $name:expr, default $default:expr, [$($values:expr),*]) => {
                input!(~ $name, DataType::Enum,
                       ValueType::Enum { values: vec![$($values.into()),*], selected: Some($default) },
                       InputParamKind::ConstantOnly)
            };
            (NewFile $name:expr) => {
                input!(~ $name, DataType::NewFile,
                       ValueType::NewFile { path: None },
                       InputParamKind::ConstantOnly)
            };
            (~ $name:expr, $data_type:expr, $value_type:expr, $param_kind:expr) => {
                graph.add_input_param(
                    node_id,
                    $name.into(),
                    $data_type,
                    $value_type,
                    $param_kind,
                    true,
                )
            };
        }

        macro_rules! output {
            (Mesh $name:expr) => { output!(~ $name, DataType::Mesh) };
            (Vector $name:expr) => { output!(~ $name, DataType::Vector) };
            (Scalar $name:expr) => { output!(~ $name, DataType::Scalar) };
            (~ $name:expr, $typ:expr) => {
                graph.add_output_param(node_id, $name.into(), $typ)
            }
        }

        match self {
            GraphNodeType::MakeBox => {
                input!(Vector "origin", Vec3::ZERO);
                input!(Vector "size", Vec3::ONE);
                output!(Mesh "out_mesh");
            }
            GraphNodeType::MakeQuad => {
                input!(Vector "center", Vec3::ZERO);
                input!(Vector "normal", Vec3::Y);
                input!(Vector "right", Vec3::X);
                input!(Vector "size", Vec3::ONE);
                output!(Mesh "out_mesh");
            }
            GraphNodeType::BevelEdges => {
                input!(Mesh "in_mesh");
                input!(Selection "edges");
                input!(Scalar "amount", 0.0, 0.0, 1.0);
                output!(Mesh "out_mesh");
            }
            GraphNodeType::ExtrudeFaces => {
                input!(Mesh "in_mesh");
                input!(Selection "faces");
                input!(Scalar "amount", 0.0, 0.0, 1.0);
                output!(Mesh "out_mesh");
            }
            GraphNodeType::ChamferVertices => {
                input!(Mesh "in_mesh");
                input!(Selection "vertices");
                input!(Scalar "amount", 0.0, 0.0, 1.0);
                output!(Mesh "out_mesh");
            }
            GraphNodeType::MakeVector => {
                input!(Scalar "x", 0.0);
                input!(Scalar "y", 0.0);
                input!(Scalar "z", 0.0);
                output!(Vector "out_vec");
            }
            GraphNodeType::VectorMath => {
                input!(Enum "vec_op", ["ADD", "SUB", "MUL"]);
                input!(Vector "A", Vec3::ZERO);
                input!(Vector "B", Vec3::ZERO);
                output!(Vector "out_vec");
            }
            GraphNodeType::MergeMeshes => {
                input!(Mesh "A");
                input!(Mesh "B");
                output!(Mesh "out_mesh");
            }
            GraphNodeType::ExportObj => {
                input!(Mesh "mesh");
                input!(NewFile "export_path");
            }
            GraphNodeType::MeshSubdivide => {
                input!(Mesh "in_mesh");
                input!(Scalar "iterations", 1.0, 1.0, 7.0);
                input!(Enum "technique", default 0, ["linear", "catmull-clark"]);
                output!(Mesh "out_mesh");
            }
        }
    }
}

pub struct AllNodeTemplates;
impl NodeTemplateIter for AllNodeTemplates {
    type Item = GraphNodeType;

    fn all_kinds(&self) -> Vec<Self::Item> {
        GraphNodeType::all_types().collect()
    }
}
