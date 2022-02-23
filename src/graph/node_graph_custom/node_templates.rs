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

pub enum InputDescriptor {
    Vector {
        default: Vec3,
    },
    Mesh,
    Selection,
    Scalar {
        default: f32,
        min: f32,
        max: f32,
    },
    Enum {
        default: Option<u32>,
        values: Vec<String>,
    },
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

macro_rules! in_vector {
    ($name:expr, $default:expr) => {
        (
            $name.to_owned(),
            InputDescriptor::Vector { default: $default },
        )
    };
}

macro_rules! in_scalar {
    ($name:expr, $default:expr, $min:expr, $max:expr) => {
        (
            $name.to_owned(),
            InputDescriptor::Scalar {
                default: $default,
                max: $max,
                min: $min,
            },
        )
    };
    ($name:expr) => {
        in_scalar!($name, 0.0, -1.0, 2.0)
    };
}

macro_rules! in_mesh {
    ($name:expr) => {
        ($name.to_owned(), InputDescriptor::Mesh)
    };
}

macro_rules! out_mesh {
    ($name:expr) => {
        ($name.to_owned(), OutputDescriptor(DataType::Mesh))
    };
}

macro_rules! out_vector {
    ($name:expr) => {
        ($name.to_owned(), OutputDescriptor(DataType::Vector))
    };
}

macro_rules! in_selection {
    ($name:expr) => {
        ($name.to_owned(), InputDescriptor::Selection)
    };
}

macro_rules! in_file {
    ($name:expr) => {
        ($name.to_owned(), InputDescriptor::NewFile)
    };
}

macro_rules! in_enum {
    ($name:expr, $( $values:expr ),+) => {
        ($name.to_owned(), InputDescriptor::Enum { default: None, values: vec![$( $values.to_owned() ),+] })
    };
    ($name:expr, default $default:expr, $( $values:expr ),+) => {
        ($name.to_owned(), InputDescriptor::Enum { default: Some($default), values: vec![$( $values.to_owned() ),+] })
    };
}

impl GraphNodeType {
    pub fn to_descriptor(&self) -> NodeDescriptor {
        let label = self.type_label().into();
        let op_name = self.op_name().into();
        match self {
            GraphNodeType::MakeBox => NodeDescriptor {
                op_name,
                label,
                inputs: vec![
                    in_vector!("origin", Vec3::ZERO),
                    in_vector!("size", Vec3::ONE),
                ],
                outputs: vec![out_mesh!("out_mesh")],
                is_executable: false,
            },
            GraphNodeType::MakeQuad => NodeDescriptor {
                op_name,
                label,
                inputs: vec![
                    in_vector!("center", Vec3::ZERO),
                    in_vector!("normal", Vec3::Y),
                    in_vector!("right", Vec3::X),
                    in_vector!("size", Vec3::ONE),
                ],
                outputs: vec![out_mesh!("out_mesh")],
                is_executable: false,
            },
            GraphNodeType::BevelEdges => NodeDescriptor {
                op_name,
                label,
                inputs: vec![
                    in_mesh!("in_mesh"),
                    in_selection!("edges"),
                    in_scalar!("amount", 0.0, 0.0, 1.0),
                ],
                outputs: vec![out_mesh!("out_mesh")],
                is_executable: false,
            },
            GraphNodeType::ExtrudeFaces => NodeDescriptor {
                op_name,
                label,
                inputs: vec![
                    in_mesh!("in_mesh"),
                    in_selection!("faces"),
                    in_scalar!("amount", 0.0, 0.0, 1.0),
                ],
                outputs: vec![out_mesh!("out_mesh")],
                is_executable: false,
            },
            GraphNodeType::ChamferVertices => NodeDescriptor {
                op_name,
                label,
                inputs: vec![
                    in_mesh!("in_mesh"),
                    in_selection!("vertices"),
                    in_scalar!("amount", 0.0, 0.0, 1.0),
                ],
                outputs: vec![out_mesh!("out_mesh")],
                is_executable: false,
            },
            GraphNodeType::MakeVector => NodeDescriptor {
                op_name,
                label,
                inputs: vec![in_scalar!("x"), in_scalar!("y"), in_scalar!("z")],
                outputs: vec![out_vector!("out_vec")],
                is_executable: false,
            },
            GraphNodeType::VectorMath => NodeDescriptor {
                op_name,
                label,
                inputs: vec![
                    in_enum!("vec_op", "ADD", "SUB", "MUL"),
                    in_vector!("A", Vec3::ZERO),
                    in_vector!("B", Vec3::ZERO),
                ],
                outputs: vec![out_vector!("out_vec")],
                is_executable: false,
            },
            GraphNodeType::MergeMeshes => NodeDescriptor {
                op_name,
                label,
                inputs: vec![in_mesh!("A"), in_mesh!("B")],
                outputs: vec![out_mesh!("out_mesh")],
                is_executable: false,
            },
            GraphNodeType::ExportObj => NodeDescriptor {
                op_name,
                label,
                inputs: vec![in_mesh!("mesh"), in_file!("export_path")],
                outputs: vec![],
                is_executable: true,
            },
            GraphNodeType::MeshSubdivide => NodeDescriptor {
                op_name,
                label,
                inputs: vec![
                    in_mesh!("in_mesh"),
                    in_scalar!("iterations", 1.0, 1.0, 7.0),
                    in_enum!("technique", default 0, "linear", "catmull-clark"),
                ],
                outputs: vec![out_mesh!("out_mesh")],
                is_executable: false,
            },
        }
    }

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
        }
    }

    fn build_node(
        &self,
        graph: &mut egui_node_graph::Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        node_id: egui_node_graph::NodeId,
    ) {
        let descriptor = self.to_descriptor();
        for input in descriptor.inputs {
            let (typ, value, kind, shown_inline) = match input.1 {
                InputDescriptor::Vector { default } => (
                    DataType::Vector,
                    ValueType::Vector(default),
                    InputParamKind::ConnectionOrConstant,
                    true,
                ),
                InputDescriptor::Mesh => (
                    DataType::Mesh,
                    ValueType::None,
                    InputParamKind::ConnectionOnly,
                    true,
                ),
                InputDescriptor::Selection => (
                    DataType::Selection,
                    ValueType::Selection {
                        text: "".into(),
                        selection: Some(vec![]),
                    },
                    InputParamKind::ConnectionOnly,
                    true,
                ),
                InputDescriptor::Scalar { default, min, max } => (
                    DataType::Scalar,
                    ValueType::Scalar {
                        value: default,
                        min,
                        max,
                    },
                    InputParamKind::ConnectionOrConstant,
                    true,
                ),
                InputDescriptor::Enum { default, values } => (
                    DataType::Enum,
                    ValueType::Enum {
                        values,
                        selection: default,
                    },
                    InputParamKind::ConstantOnly,
                    true,
                ),
                InputDescriptor::NewFile => (
                    DataType::NewFile,
                    ValueType::NewFile { path: None },
                    InputParamKind::ConstantOnly,
                    true,
                ),
            };
            graph.add_input_param(node_id, input.0, typ, value, kind, shown_inline);
        }

        for output in descriptor.outputs {
            graph.add_output_param(node_id, output.0, output.1 .0);
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
