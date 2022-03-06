use std::sync::Arc;

use crate::{
    prelude::*,
    rendergraph::{
        grid_routine::GridRoutine, point_cloud_routine::PointCloudRoutine,
        shader_manager::ShaderManager, wireframe_routine::WireframeRoutine, face_routine::FaceRoutine,
    },
};

use glam::Mat4;
use rend3_routine::pbr::PbrRoutine;
use wgpu::{Surface, TextureFormat};

pub struct RenderContext {
    pub renderer: Arc<r3::Renderer>,

    pub base_graph: r3::BaseRenderGraph,
    pub pbr_routine: r3::PbrRoutine,
    pub tonemapping_routine: r3::TonemappingRoutine,
    pub grid_routine: GridRoutine,
    pub wireframe_routine: WireframeRoutine,
    pub face_routine: FaceRoutine,
    pub point_cloud_routine: PointCloudRoutine,
    pub surface: Arc<Surface>,
    pub texture_format: TextureFormat,
    pub shader_manager: ShaderManager,

    pub objects: Vec<r3::ObjectHandle>,
    lights: Vec<r3::DirectionalLightHandle>,
}

impl RenderContext {
    pub fn new(window: &winit::window::Window) -> Self {
        let window_size = window.inner_size();
        let iad = pollster::block_on(rend3::create_iad(
            None,
            None,
            Some(rend3::RendererProfile::CpuDriven),
            None,
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

        let shader_manager = ShaderManager::new(&renderer.device);
        let grid_routine = GridRoutine::new(&renderer.device);
        let wireframe_routine =
            WireframeRoutine::new(&renderer.device, &base_graph, &shader_manager);
        let point_cloud_routine =
            PointCloudRoutine::new(&renderer.device, &base_graph, &shader_manager);
        let face_routine =
            FaceRoutine::new(&renderer.device, &base_graph, &shader_manager);

        RenderContext {
            renderer,
            pbr_routine,
            base_graph,
            tonemapping_routine,
            grid_routine,
            wireframe_routine,
            point_cloud_routine,
            face_routine,
            surface,
            texture_format: format,
            shader_manager,
            objects: vec![],
            lights: vec![],
        }
    }

    pub fn clear_objects(&mut self) {
        self.objects.clear();
        self.point_cloud_routine.clear();
        self.wireframe_routine.clear();
        self.face_routine.clear();
    }

    pub fn add_mesh_as_object<M: r3::Material>(&mut self, mesh: r3::Mesh, material: Option<M>) {
        let mesh_handle = self.renderer.add_mesh(mesh);
        let material_handle = if let Some(material) = material {
            self.renderer.add_material(material)
        } else {
            let material = r3::PbrMaterial {
                albedo: r3::AlbedoComponent::Value(glam::Vec4::new(0.8, 0.1, 0.1, 1.0)),
                ..Default::default()
            };
            self.renderer.add_material(material)
        };
        let object = r3::Object {
            mesh_kind: r3::ObjectMeshKind::Static(mesh_handle),
            material: material_handle,
            transform: glam::Mat4::IDENTITY,
        };
        self.objects.push(self.renderer.add_object(object));
    }

    pub fn add_object(&mut self, object: r3::Object) {
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

    #[allow(dead_code)]
    pub fn project_point(&self, point: Vec3, screen_size: Vec2) -> Vec2 {
        let camera_manager = &self.renderer.data_core.lock().camera_manager;

        let clip = camera_manager.view_proj().project_point3(point);
        let clip = Vec2::new(clip.x, -clip.y);
        let zero_to_one = (Vec2::new(clip.x, clip.y) + Vec2::ONE) * 0.5;
        zero_to_one * screen_size
    }

    pub fn add_light(&mut self, light: r3::DirectionalLight) {
        let handle = self.renderer.add_directional_light(light);
        self.lights.push(handle);
    }

    pub fn on_resize(&mut self, width: u32, height: u32) {
        rend3::configure_surface(
            &self.surface,
            &self.renderer.device,
            self.texture_format,
            glam::uvec2(width, height),
            rend3::types::PresentMode::Mailbox,
        );
    }
}
