use std::sync::Arc;

use crate::{prelude::*, rendergraph::GridRoutine};

use glam::Mat4;
use rend3::{
    types::{DirectionalLight, Mesh, Object, ResourceHandle, SampleCount},
    Renderer,
};
use rend3_egui::EguiRenderRoutine;
use rend3_routine::{PbrRenderRoutine, TonemappingRoutine};
use wgpu::{Features, Surface, TextureFormat};

use crate::rendergraph::{self, wireframe_pass::WireframeRoutine};

pub struct RenderContext {
    pub renderer: Arc<Renderer>,
    pub pbr_routine: PbrRenderRoutine,
    pub tonemapping_routine: TonemappingRoutine,
    pub wireframe_routine: WireframeRoutine,
    pub egui_routine: EguiRenderRoutine,
    pub grid_routine: GridRoutine,
    pub surface: Arc<Surface>,
    pub texture_format: TextureFormat,

    pub objects: Vec<ResourceHandle<Object>>,
    lights: Vec<ResourceHandle<DirectionalLight>>,
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

        let renderer = rend3::Renderer::new(
            iad,
            Some(window_size.width as f32 / window_size.height as f32),
        )
        .unwrap();

        let render_texture_options = rend3_routine::RenderTextureOptions {
            resolution: glam::UVec2::new(window_size.width, window_size.height),
            samples: SampleCount::One,
        };
        let mut pbr_routine =
            rend3_routine::PbrRenderRoutine::new(&renderer, render_texture_options);
        let tonemapping_routine = rend3_routine::TonemappingRoutine::new(
            &renderer,
            render_texture_options.resolution,
            format,
        );

        let egui_routine = EguiRenderRoutine::new(
            &renderer,
            format,
            SampleCount::One,
            window_size.width,
            window_size.height,
            window.scale_factor() as f32,
        );

        let grid_routine = GridRoutine::new(&renderer.device);

        let wireframe_routine = WireframeRoutine::new(&renderer.device, &pbr_routine);

        pbr_routine.set_ambient_color(glam::Vec4::ONE * 0.25);

        RenderContext {
            renderer,
            pbr_routine,
            tonemapping_routine,
            egui_routine,
            wireframe_routine,
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
        let material = rend3_routine::material::PbrMaterial {
            albedo: rend3_routine::material::AlbedoComponent::Value(glam::Vec4::new(
                0.8, 0.1, 0.1, 1.0,
            )),
            ..rend3_routine::material::PbrMaterial::default()
        };
        let material_handle = self.renderer.add_material(material);
        let object = rend3::types::Object {
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
        let camera_manager = self.renderer.camera_manager.read();

        let clip = camera_manager.view_proj().project_point3(point);
        let clip = Vec2::new(clip.x, -clip.y);
        let zero_to_one = (Vec2::new(clip.x, clip.y) + Vec2::ONE) * 0.5;
        zero_to_one * screen_size
    }

    pub fn add_light(&mut self, light: DirectionalLight) {
        let handle = self.renderer.add_directional_light(light);
        self.lights.push(handle);
    }

    pub fn render_frame(&mut self, egui_platform: Option<&mut egui_winit_platform::Platform>) {
        let frame = rend3::util::output::OutputFrame::Surface {
            surface: Arc::clone(&self.surface),
        };
        let (cmd_bufs, ready) = self.renderer.ready();

        let egui_paint_jobs;

        let mut graph = rend3::RenderGraph::new();

        rendergraph::add_default_rendergraph(
            &mut graph,
            &ready,
            &self.pbr_routine,
            None,
            &self.tonemapping_routine,
            &self.wireframe_routine,
            &self.grid_routine,
            rend3::types::SampleCount::One,
        );

        if let Some(platform) = egui_platform {
            let (_output, paint_commands) = platform.end_frame(None);
            egui_paint_jobs = platform.context().tessellate(paint_commands);
            let input = rend3_egui::Input {
                clipped_meshes: &egui_paint_jobs,
                context: platform.context(),
            };

            let surface = graph.add_surface_texture();
            self.egui_routine.add_to_graph(&mut graph, input, surface);
        };

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

        self.renderer.set_aspect_ratio(width as f32 / height as f32 * 2.0);

        let size = UVec2::new(width, height);
        let options = rend3_routine::RenderTextureOptions {
            resolution: size,
            samples: SampleCount::One,
        };

        self.pbr_routine.resize(&self.renderer, options);
        self.tonemapping_routine.resize(size);
        self.egui_routine.resize(width, height, scale_factor);
    }
}
