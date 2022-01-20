use std::{rc::Rc, sync::Arc};

use crate::{
    graph::graph_editor_egui::viewport_manager::AppViewports, prelude::*,
    rendergraph::grid_routine::GridRoutine,
};

use glam::Mat4;
use rend3::{
    types::{DirectionalLight, Mesh, Object, ResourceHandle, SampleCount},
    Renderer,
};
use rend3_egui::EguiRenderRoutine;
use rend3_routine::pbr::PbrRoutine;
use wgpu::{Features, Surface, TextureFormat};

use crate::rendergraph;

pub struct RenderContext {
    pub renderer: Arc<Renderer>,

    pub base_graph: r3::BaseRenderGraph,
    pub pbr_routine: r3::PbrRoutine,
    pub tonemapping_routine: r3::TonemappingRoutine,
    /// The egui routine responsible for drawing the main application UI
    pub main_egui_routine: rendergraph::egui_routine_custom::EguiCustomRoutine,
    /// The egui routine responsible for drawing the graph editor. This is
    /// renderd to an offscreen texture so we can do pan / zoom.
    pub graph_egui_routine: rendergraph::egui_routine_custom::EguiCustomRoutine,
    pub grid_routine: GridRoutine,
    pub surface: Arc<Surface>,
    pub texture_format: TextureFormat,

    pub objects: Vec<ResourceHandle<Object>>,
    lights: Vec<ResourceHandle<DirectionalLight>>,
}

fn ambient_light() -> Vec4 {
    Vec4::ONE * 0.25
}

impl RenderContext {
    pub fn new(window: &winit::window::Window) -> Self {
        let window_size = window.inner_size();
        let iad = pollster::block_on(rend3::create_iad(
            None,
            None,
            None,
            Some(Features::POLYGON_MODE_LINE),
        ))
        .unwrap();

        let surface = Arc::new(unsafe { iad.instance.create_surface(&window) });

        let format = surface.get_preferred_format(&iad.adapter).unwrap();
        rend3::configure_surface(
            &surface,
            &iad.device,
            format,
            glam::UVec2::new(window_size.width, window_size.height),
            rend3::types::PresentMode::Mailbox,
        );

        let renderer = r3::Renderer::new(
            iad,
            r3::Handedness::Left,
            Some(window_size.width as f32 / window_size.height as f32),
        )
        .unwrap();

        let base_graph = r3::BaseRenderGraph::new(&renderer);
        let mut data_core = renderer.data_core.lock();
        let pbr_routine = PbrRoutine::new(&renderer, &mut data_core, &base_graph.interfaces);
        let tonemapping_routine =
            r3::TonemappingRoutine::new(&renderer, &base_graph.interfaces, format);
        drop(data_core); // Release the lock

        let main_egui_routine = rendergraph::egui_routine_custom::EguiCustomRoutine::new(
            &renderer,
            format,
            SampleCount::One,
            window_size.width,
            window_size.height,
            window.scale_factor() as f32,
        );

        let graph_egui_routine = rendergraph::egui_routine_custom::EguiCustomRoutine::new(
            &renderer,
            format,
            SampleCount::One,
            window_size.width,
            window_size.height,
            window.scale_factor() as f32,
        );

        let grid_routine = GridRoutine::new(&renderer.device);

        RenderContext {
            renderer,
            pbr_routine,
            base_graph,
            tonemapping_routine,
            main_egui_routine,
            graph_egui_routine,
            grid_routine,
            surface,
            texture_format: format,
            objects: vec![],
            lights: vec![],
        }
    }

    pub fn clear_objects(&mut self) {
        self.objects.clear();
    }

    pub fn add_mesh_as_object(&mut self, mesh: Mesh) {
        let mesh_handle = self.renderer.add_mesh(mesh);
        let material = r3::PbrMaterial {
            albedo: r3::AlbedoComponent::Value(glam::Vec4::new(0.8, 0.1, 0.1, 1.0)),
            ..Default::default()
        };
        let material_handle = self.renderer.add_material(material);
        let object = r3::Object {
            mesh: mesh_handle,
            material: material_handle,
            transform: glam::Mat4::IDENTITY,
        };
        self.objects.push(self.renderer.add_object(object));
    }

    pub fn add_object(&mut self, object: Object) {
        self.objects.push(self.renderer.add_object(object));
    }

    pub fn set_camera(&mut self, view_matrix: Mat4) {
        self.renderer.set_camera_data(rend3::types::Camera {
            projection: rend3::types::CameraProjection::Perspective {
                vfov: 60.0,
                near: 0.1,
            },
            view: view_matrix,
        });
    }

    pub fn project_point(&self, point: Vec3, screen_size: Vec2) -> Vec2 {
        let camera_manager = &self.renderer.data_core.lock().camera_manager;

        let clip = camera_manager.view_proj().project_point3(point);
        let clip = Vec2::new(clip.x, -clip.y);
        let zero_to_one = (Vec2::new(clip.x, clip.y) + Vec2::ONE) * 0.5;
        zero_to_one * screen_size
    }

    pub fn add_light(&mut self, light: DirectionalLight) {
        let handle = self.renderer.add_directional_light(light);
        self.lights.push(handle);
    }

    pub fn render_frame(
        &mut self,
        main_egui: &mut egui_winit_platform::Platform,
        graph_egui: &mut egui_winit_platform::Platform,
        app_viewports: &mut AppViewports,
    ) {
        let frame = rend3::util::output::OutputFrame::Surface {
            surface: Arc::clone(&self.surface),
        };
        let (cmd_bufs, ready) = self.renderer.ready();

        let main_egui_paint_jobs;
        let graph_egui_paint_jobs;

        let mut graph = rend3::RenderGraph::new();

        let vwp_3d_res = app_viewports.view_3d.rect.size();
        let grph_3d_res = app_viewports.node_graph.rect.size();
        let to_uvec2 = |v: egui::Vec2| UVec2::new(v.x as u32, v.y as u32);

        // TODO: What if we ever have multiple 3d viewports? There's no way to
        // set the aspect ratio differently for different render passes in rend3
        // right now. The camera is global.
        //
        // See: https://github.com/BVE-Reborn/rend3/issues/327
        self.renderer.set_aspect_ratio(vwp_3d_res.x / vwp_3d_res.y);

        let viewport_texture = rendergraph::blackjack_viewport_rendergraph(
            &self.base_graph,
            &mut graph,
            &ready,
            &self.pbr_routine,
            &self.tonemapping_routine,
            &self.grid_routine,
            // The resolution needs to be scaled by the pixels-per-point
            to_uvec2(vwp_3d_res * main_egui.context().pixels_per_point()),
            r3::SampleCount::One,
            ambient_light(),
        );

        self.graph_egui_routine.resize(
            app_viewports.node_graph.rect.width() as u32,
            app_viewports.node_graph.rect.height() as u32,
            1.0,
        );
        let graph_egui_texture = {
            let graph_egui_render_target = graph.add_render_target(r3::RenderTargetDescriptor {
                label: None,
                resolution: to_uvec2(grph_3d_res),
                samples: r3::SampleCount::One,
                format: r3::TextureFormat::Bgra8UnormSrgb,
                usage: r3::TextureUsages::RENDER_ATTACHMENT | r3::TextureUsages::TEXTURE_BINDING,
            });
            let (_output, paint_commands) = graph_egui.end_frame(None);
            graph_egui_paint_jobs = graph_egui.context().tessellate(paint_commands);
            let graph_egui_input = rendergraph::egui_routine_custom::Input {
                clipped_meshes: &graph_egui_paint_jobs,
                context: graph_egui.context(),
            };
            self.graph_egui_routine.add_sub_ui_to_graph(
                &mut graph,
                graph_egui_input,
                graph_egui_render_target,
            );
            graph_egui_render_target
        };

        {
            let (_output, paint_commands) = main_egui.end_frame(None);
            main_egui_paint_jobs = main_egui.context().tessellate(paint_commands);
            let main_egui_input = rendergraph::egui_routine_custom::Input {
                clipped_meshes: &main_egui_paint_jobs,
                context: main_egui.context(),
            };
            let surface = graph.add_surface_texture();
            self.main_egui_routine.add_main_egui_to_graph(
                &mut graph,
                main_egui_input,
                surface,
                viewport_texture,
                graph_egui_texture,
                app_viewports,
            );
        }

        graph.execute(&self.renderer, frame, cmd_bufs, &ready);
    }

    pub fn on_resize(&mut self, width: u32, height: u32, scale_factor: f32) {
        rend3::configure_surface(
            &self.surface,
            &self.renderer.device,
            self.texture_format,
            glam::uvec2(width, height),
            rend3::types::PresentMode::Mailbox,
        );

        self.renderer
            .set_aspect_ratio(width as f32 / height as f32 * 2.0);

        let size = UVec2::new(width, height);
        self.main_egui_routine.resize(width, height, scale_factor);
    }
}
