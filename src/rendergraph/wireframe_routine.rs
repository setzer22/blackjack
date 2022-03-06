use super::viewport_3d_routine::{Viewport3dRoutine, ViewportBuffers};
use crate::prelude::r3;
use glam::Vec3;
use rend3_routine::base::{BaseRenderGraph, BaseRenderGraphIntermediateState};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use super::shader_manager::ShaderManager;

/// Stores a wgpu buffer containing the edges of a wireframe
pub struct WireframeBuffer {
    /// Contains 2*len Vec3 elements
    line_positions: Buffer,
    /// Contains len Vec3 elements (color)
    colors: Buffer,
    /// Number of elements
    len: usize,
}

impl ViewportBuffers for WireframeBuffer {
    const NUM_BUFFERS: usize = 2;

    fn get_wgpu_buffers(&self) -> Vec<&Buffer> {
        vec![&self.line_positions, &self.colors]
    }

    fn vertex_instance_counts(&self) -> (u32, u32) {
        (2, self.len as u32)
    }
}

pub struct WireframeRoutine {
    inner: Viewport3dRoutine<WireframeBuffer>,
}

impl WireframeRoutine {
    pub fn new(device: &Device, base: &BaseRenderGraph, shader_manager: &ShaderManager) -> Self {
        Self {
            inner: Viewport3dRoutine::new(
                "edge wireframe",
                device,
                base,
                shader_manager.get("edge_wireframe_draw"),
                PrimitiveTopology::LineList,
                FrontFace::Ccw,
            ),
        }
    }

    pub fn add_wireframe(&mut self, device: &Device, lines: &[Vec3], colors: &[Vec3]) {
        let len = colors.len();
        assert!(
            lines.len() == colors.len() * 2,
            "There must be exactly 2*N lines and N colors in a wireframe"
        );

        let line_positions = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(lines),
            usage: BufferUsages::STORAGE,
        });
        let colors = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(colors),
            usage: BufferUsages::STORAGE,
        });

        self.inner.buffers.push(WireframeBuffer {
            len,
            line_positions,
            colors,
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
