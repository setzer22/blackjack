use anyhow::Error;

use crate::{
    graph::graph_editor_egui::{editor_state::GraphEditorState, viewport_split::SplitTree},
    prelude::debug_viz::DebugMeshes,
    prelude::*,
};

pub struct ApplicationContext {
    /// The mesh is at the center of the application
    /// - The graph generates a program that produces this mesh.
    /// - The 3d viewport renders this mesh.
    pub mesh: Option<HalfEdgeMesh>,
    /// The tree of splits at the center of application. Splits recursively
    /// partition the state either horizontally or vertically. This separation
    /// is dynamic, very similar to Blender's UI model
    pub split_tree: SplitTree,
    /// When set, the file path stored in the inner string will be loaded.
    pub load_op: Option<String>,

    pub debug_meshes: DebugMeshes,
}

impl ApplicationContext {
    pub fn new(renderer: &r3::Renderer) -> ApplicationContext {
        ApplicationContext {
            mesh: None,
            split_tree: SplitTree::default_tree(),
            load_op: None,
            debug_meshes: DebugMeshes::new(renderer),
        }
    }

    pub fn setup(&self, render_ctx: &mut RenderContext) {
        render_ctx.add_light(r3::DirectionalLight {
            color: glam::Vec3::ONE,
            intensity: 10.0,
            // Direction will be normalized
            direction: glam::Vec3::new(-1.0, -4.0, 2.0),
            distance: 400.0,
        });
    }

    pub fn update(
        &mut self,
        egui_ctx: &egui::CtxRef,
        editor_state: &GraphEditorState,
        render_ctx: &mut RenderContext,
    ) {
        // TODO: Instead of clearing all objects, make the app context own the
        // objects it's drawing and clear those instead.
        render_ctx.clear_objects();

        if let Err(err) = self.compile_and_update_mesh(editor_state) {
            self.paint_errors(egui_ctx, err);
        }
        self.build_and_render_mesh(render_ctx);
    }

    pub fn build_and_render_mesh(&mut self, render_ctx: &mut RenderContext) {
        if let Some(mesh) = self.mesh.as_ref() {
            self.debug_meshes.add_halfedge_debug(render_ctx, mesh);

            let (positions, indices) = mesh.generate_buffers();
            let r3_mesh = r3::MeshBuilder::new(positions, r3::Handedness::Left)
                .with_indices(indices)
                .build()
                .unwrap();
            render_ctx.add_mesh_as_object(r3_mesh);
        }
    }

    pub fn paint_errors(&mut self, egui_ctx: &egui::CtxRef, err: Error) {
        let painter = egui_ctx.debug_painter();
        let width = egui_ctx.available_rect().width();
        painter.text(
            egui::pos2(width - 10.0, 30.0),
            egui::Align2::RIGHT_TOP,
            format!("{}", err),
            egui::TextStyle::Body,
            egui::Color32::RED,
        );
    }

    pub fn compile_and_update_mesh(&mut self, editor_state: &GraphEditorState) -> Result<()> {
        if let Some(active) = editor_state.active_node {
            let program = crate::graph::graph_compiler::compile_graph(&editor_state.graph, active)?;
            let mesh = program.execute()?;
            self.mesh = Some(mesh);
        } else {
            self.mesh = None
        }
        Ok(())
    }
}
