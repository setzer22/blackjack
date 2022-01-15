
/// The main type definitions of the graph data structure
pub mod graph_types;

/// The egui-based node editor
pub mod graph_editor_egui;

/// A list of instructions to procedurally generate a mesh
pub mod poly_asm;

/// Compiles node graphs into PolyAsm programs
pub mod graph_compiler;