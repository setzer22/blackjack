use super::*;
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
