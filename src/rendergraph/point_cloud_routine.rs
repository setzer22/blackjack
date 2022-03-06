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

pub struct PointCloudBuffer {
    buffer: Buffer,
    len: usize,
}

impl ViewportBuffers for PointCloudBuffer {
    const NUM_BUFFERS: usize = 1;

    fn get_wgpu_buffers(&self) -> Vec<&Buffer> {
        vec![&self.buffer]
    }

    fn vertex_instance_counts(&self) -> (u32, u32) {
        // 6 vertices to render quads
        (6, self.len as u32)
    }
}

pub struct PointCloudRoutine {
    inner: Viewport3dRoutine<PointCloudBuffer>,
}

impl PointCloudRoutine {
    pub fn new(device: &Device, base: &BaseRenderGraph, shader_manager: &ShaderManager) -> Self {
        Self {
            inner: Viewport3dRoutine::new(
                "point cloud",
                device,
                base,
                shader_manager.get("point_cloud_draw"),
                PrimitiveTopology::TriangleList,
                FrontFace::Ccw,
            ),
        }
    }

    pub fn add_point_cloud(&mut self, device: &Device, points: &[Vec3]) {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(points),
            usage: BufferUsages::STORAGE,
        });
        self.inner.buffers.push(PointCloudBuffer {
            buffer,
            len: points.len(),
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
