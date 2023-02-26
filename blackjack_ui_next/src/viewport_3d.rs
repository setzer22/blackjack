use std::{cell::Cell, sync::Arc};

use blackjack_engine::prelude::primitives;
use egui_wgpu::RenderState;
use glam::UVec2;
use guee::{base_widgets::image::Image, callback_accessor::CallbackAccessor, prelude::*};

use crate::renderer::{BlackjackViewportRenderer, ViewportCamera};

use self::orbit_camera::OrbitCamera;

pub mod lerp;

pub mod orbit_camera;

#[derive(PartialEq, Eq, Default)]
pub enum EdgeDrawMode {
    HalfEdge,
    #[default]
    FullEdge,
    NoDraw,
}

#[derive(PartialEq, Eq, Default)]
pub enum FaceDrawMode {
    /// Will read the actual configured value for the mesh and use its channel,
    /// if any. Defaults to flat shading otherwise.
    #[default]
    Real,
    /// Force flat shading, ignoring mesh data.
    Flat,
    /// Force smooth shading, ignoring mesh data
    Smooth,
    /// Don't draw faces.
    NoDraw,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TextOverlayMode {
    /// No text overlay
    #[default]
    NoDraw,
    /// Display face ids
    MeshInfoFaces,
    /// Display vertex ids
    MeshInfoVertices,
    /// Display halfedge ids
    MeshInfoHalfedges,
    /// Display all edge ids
    MeshInfoAll,
    /// Display mesh debug information set by the developers when debugging a
    /// problem. This is not intended to be used by regular users.
    DevDebug,
}

pub struct Viewport3dSettings {
    pub render_vertices: bool,
    pub matcap: usize,
    pub edge_mode: EdgeDrawMode,
    pub face_mode: FaceDrawMode,
    pub overlay_mode: TextOverlayMode,
}

impl Default for Viewport3dSettings {
    fn default() -> Self {
        Self {
            render_vertices: true,
            matcap: 0,
            edge_mode: Default::default(),
            face_mode: Default::default(),
            overlay_mode: Default::default(),
        }
    }
}

pub struct Viewport3d {
    pub renderer: BlackjackViewportRenderer,
    pub settings: Viewport3dSettings,
    pub last_frame_bounds: Option<Rect>,
    pub epaint_texture_id: Cell<Option<TextureId>>,
    pub cba: CallbackAccessor<Self>,
    pub camera: OrbitCamera,
}

impl Viewport3d {
    pub fn new(render_ctx: &RenderState, cba: CallbackAccessor<Self>) -> Self {
        Self {
            renderer: BlackjackViewportRenderer::new(
                Arc::clone(&render_ctx.device),
                Arc::clone(&render_ctx.queue),
            ),
            settings: Default::default(),
            // We render with 1 frame delay to know the size of the UI element
            last_frame_bounds: None,
            epaint_texture_id: Cell::new(None),
            cba,
            camera: OrbitCamera::default(),
        }
    }

    pub fn view(&self, render_ctx: &RenderState) -> DynWidget {
        if let Some(last_frame_bounds) = self.last_frame_bounds {
            let resolution = UVec2::new(
                last_frame_bounds.width() as u32,
                last_frame_bounds.height() as u32,
            );
            let output = self.renderer.render(
                resolution,
                self.camera.compute_matrices(resolution),
                &self.settings,
            );
            if let Some(tex_id) = self.epaint_texture_id.get() {
                render_ctx
                    .renderer
                    .write()
                    .update_egui_texture_from_wgpu_texture(
                        &render_ctx.device,
                        &output.color_texture_view,
                        wgpu::FilterMode::Linear,
                        tex_id,
                    );
            } else {
                self.epaint_texture_id.set(Some(
                    render_ctx.renderer.write().register_native_texture(
                        &render_ctx.device,
                        &output.color_texture_view,
                        wgpu::FilterMode::Linear,
                    ),
                ));
            }
        }

        let image = if let Some(tex_id) = self.epaint_texture_id.get() {
            Image::new(IdGen::key("viewport"), tex_id, LayoutHints::fill()).build()
        } else {
            // For the first frame, just render background
            ColoredBox::background(/*pallette().background_dark*/ color!("#ff0000"))
                .hints(LayoutHints::fill())
                .build()
        };

        let set_last_frame_res_cb = self.cba.callback(|viewport, new_bounds: Rect| {
            viewport.last_frame_bounds = Some(new_bounds);
        });
        EventHandlingContainer::new(image)
            .pre_event(|ctx, layout, _, _| {
                ctx.dispatch_callback(set_last_frame_res_cb, layout.bounds);
                EventStatus::Ignored
            })
            .build()
    }

    pub fn update(&mut self) {
        self.camera.update(1.0 / 60.0);
        let mesh = primitives::Box::build(glam::Vec3::ZERO, glam::Vec3::ONE);
        let face_bufs = mesh.generate_triangle_buffers_flat(true).unwrap();
        self.renderer.face_routine.clear();
        self.renderer.face_routine.add_base_mesh(
            &self.renderer.device,
            &face_bufs.positions,
            &face_bufs.normals,
            &face_bufs.indices,
        );

        let vertex_bufs = mesh.generate_point_buffers();
        self.renderer.point_cloud_routine.clear();
        self.renderer
            .point_cloud_routine
            .add_point_cloud(&self.renderer.device, &vertex_bufs.positions);

        let edge_bufs = mesh.generate_line_buffers().unwrap();
        self.renderer.wireframe_routine.clear();
        self.renderer.wireframe_routine.add_wireframe(
            &self.renderer.device,
            &edge_bufs.positions,
            &edge_bufs.colors,
        );
    }
}
