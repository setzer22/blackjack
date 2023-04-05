use std::{cell::Cell, rc::Rc, sync::Arc};

use blackjack_engine::lua_engine::RenderableThing;
use egui_wgpu::RenderState;
use glam::UVec2;
use guee::{
    base_widgets::image::Image, callback_accessor::CallbackAccessor, input::MouseButton, prelude::*,
};
use winit::event::VirtualKeyCode;

use crate::{
    blackjack_theme::pallette,
    icon_management::IconAtlas,
    renderer::{
        routine_renderer::MultisampleConfig, texture_manager::TextureManager,
        BlackjackViewportRenderer,
    },
};

use self::orbit_camera::{CameraInput, OrbitCamera};

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
    pub fn new(
        render_ctx: &RenderState,
        cba: CallbackAccessor<Self>,
        texture_manager: &mut TextureManager,
    ) -> Self {
        Self {
            renderer: BlackjackViewportRenderer::new(
                Arc::clone(&render_ctx.device),
                Arc::clone(&render_ctx.queue),
                MultisampleConfig::Four,
                texture_manager,
            ),
            settings: Default::default(),
            // We render with 1 frame delay to know the size of the UI element
            last_frame_bounds: None,
            epaint_texture_id: Cell::new(None),
            cba,
            camera: OrbitCamera::default(),
        }
    }

    pub fn view(&self, render_ctx: &RenderState, texture_manager: &TextureManager) -> DynWidget {
        if let Some(last_frame_bounds) = self.last_frame_bounds {
            let resolution = UVec2::new(
                last_frame_bounds.width() as u32,
                last_frame_bounds.height() as u32,
            );
            let output = self.renderer.render(
                resolution,
                self.camera.compute_matrices(resolution),
                &self.settings,
                texture_manager,
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
            ColoredBox::background(pallette().background_dark)
                .hints(LayoutHints::fill())
                .build()
        };

        let set_last_frame_res_cb = self.cba.callback(|viewport, new_bounds: Rect| {
            viewport.last_frame_bounds = Some(new_bounds);
        });
        let camera_input_cb = self.cba.callback(|viewport, cam_input| {
            viewport.camera.on_input(cam_input);
        });
        TinkerContainer::new(image)
            .post_layout(|ctx, layout| {
                ctx.dispatch_callback(set_last_frame_res_cb, layout.bounds);
            })
            .pre_event(|ctx, layout, cursor_pos, events, status| {
                if status.is_consumed() {
                    return;
                }
                let mut cam_input = CameraInput::default();
                if layout.bounds.contains(cursor_pos) {
                    cam_input.shift_down = ctx.input_state.modifiers.shift;
                    if ctx.claim_drag_event(layout.widget_id, layout.bounds, MouseButton::Primary) {
                        status.consume_event();
                        cam_input.lmb_pressed = true
                    }
                    cam_input.cursor_delta = ctx.input_state.mouse.delta();
                    for event in events {
                        match &event {
                            Event::MouseWheel(wheel_delta) => {
                                if wheel_delta.y.abs() > 0.0 {
                                    status.consume_event();
                                    cam_input.wheel_delta = wheel_delta.y;
                                }
                            }
                            Event::KeyPressed(VirtualKeyCode::F) => {
                                cam_input.f_pressed = true;
                                status.consume_event();
                            }
                            _ => (),
                        }
                    }
                }
                ctx.dispatch_callback(camera_input_cb, cam_input);
            })
            .build()
    }

    pub fn update(&mut self, renderable: Option<RenderableThing>) {
        self.camera.update(10.0 / 60.0);

        self.renderer.face_routine.clear();
        self.renderer.point_cloud_routine.clear();
        self.renderer.wireframe_routine.clear();

        match renderable {
            Some(RenderableThing::HalfEdgeMesh(mesh)) => {
                let face_bufs = mesh.generate_triangle_buffers_flat(true).unwrap();
                self.renderer.face_routine.add_base_mesh(
                    &self.renderer.device,
                    &face_bufs.positions,
                    &face_bufs.normals,
                    &face_bufs.indices,
                );

                let vertex_bufs = mesh.generate_point_buffers();
                self.renderer
                    .point_cloud_routine
                    .add_point_cloud(&self.renderer.device, &vertex_bufs.positions);

                let edge_bufs = mesh.generate_line_buffers().unwrap();
                self.renderer.wireframe_routine.add_wireframe(
                    &self.renderer.device,
                    &edge_bufs.positions,
                    &edge_bufs.colors,
                );
            }
            Some(RenderableThing::HeightMap(_)) => todo!(),
            None => (),
        }
    }
}
