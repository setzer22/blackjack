use crate::{graph::graph_editor_egui::viewport_split::SplitTree, prelude::*};

pub struct ApplicationContext {
    /// The mesh is at the center of the application
    /// - The graph generates a program that produces this mesh.
    /// - The 3d viewport renders this mesh.
    pub mesh: Option<HalfEdgeMesh>,
    /// The tree of splits at the center of application. Splits recursively
    /// partition the state either horizontally or vertically. This separation
    /// is dynamic, very similar to Blender's UI model
    pub split_tree: SplitTree,
}

impl ApplicationContext {
    pub fn new() -> ApplicationContext {
        ApplicationContext {
            mesh: None,
            split_tree: SplitTree::default_tree(),
        }
    }
}
