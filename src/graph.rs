/// Provides implementations to the traits in `egui_node_graph` specific to blackjack
pub mod node_graph_custom;

/// A list of instructions to procedurally generate a mesh
pub mod poly_asm;

/// Compiles node graphs into PolyAsm programs
pub mod graph_compiler;

/// Compiles node graphs into Lua
pub mod graph_compiler2;
