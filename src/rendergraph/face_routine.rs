use crate::prelude::r3;
use glam::Vec3;

use rend3_routine::base::{BaseRenderGraph, BaseRenderGraphIntermediateState};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use super::{
    shader_manager::ShaderManager,
    viewport_3d_routine::{Viewport3dRoutine, ViewportBuffers},
};

pub struct FacesBuffer {
    positions: Buffer,
    normals: Buffer,
    colors: Buffer,
    len: usize,
}

impl ViewportBuffers for FacesBuffer {
    const NUM_BUFFERS: usize = 3;

    fn get_wgpu_buffers(&self) -> Vec<&Buffer> {
        vec![&self.positions, &self.normals, &self.colors]
    }

    fn vertex_instance_counts(&self) -> (u32, u32) {
        // 6 vertices to render quads
        (3, self.len as u32)
    }
}

pub struct FaceRoutine {
    inner: Viewport3dRoutine<FacesBuffer>,
}

impl FaceRoutine {
    pub fn new(device: &Device, base: &BaseRenderGraph, shader_manager: &ShaderManager) -> Self {
        Self {
            inner: Viewport3dRoutine::new(
                "face",
                device,
                base,
                shader_manager.get("face_draw"),
                PrimitiveTopology::TriangleList,
                FrontFace::Cw,
            ),
        }
    }

    pub fn add_faces(&mut self, device: &Device, positions: &[Vec3], normals: &[Vec3], colors: &[Vec3]) {
        let len = colors.len();
        assert!(
            positions.len() == colors.len() * 3,
            "There must be exactly 3*N positions and N colors in a face mesh"
        );
        let positions = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(positions),
            usage: BufferUsages::STORAGE,
        });
        let normals = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(normals),
            usage: BufferUsages::STORAGE,
        });
        let colors = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(colors),
            usage: BufferUsages::STORAGE,
        });
        self.inner.buffers.push(FacesBuffer {
            positions,
            normals,
            colors,
            len,
        });
    }

    pub fn clear(&mut self) {
        self.inner.clear()
    }

    pub fn add_to_graph<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        state: &BaseRenderGraphIntermediateState,
    ) {
        self.inner.add_to_graph(graph, state);
    }
}
