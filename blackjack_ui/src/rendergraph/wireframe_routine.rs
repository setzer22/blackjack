// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::viewport_3d_routine::{DrawType, RoutineLayout, Viewport3dRoutine};
use crate::prelude::r3;
use glam::Vec3;
use rend3::managers::TextureManager;
use rend3_routine::base::{BaseRenderGraph, BaseRenderGraphIntermediateState};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use super::shader_manager::ShaderManager;

/// Stores a wgpu buffer containing the edges of a wireframe
pub struct WireframeLayout {
    /// Contains 2*len Vec3 elements
    line_positions: Buffer,
    /// Contains len Vec3 elements (color)
    colors: Buffer,
    /// Number of elements
    len: usize,
}

const NUM_BUFFERS: usize = 2;

impl RoutineLayout<NUM_BUFFERS> for WireframeLayout {
    type Settings = ();
    fn get_wgpu_buffers(&self, _settings: &()) -> [&Buffer; NUM_BUFFERS] {
        [&self.line_positions, &self.colors]
    }

    fn get_wgpu_textures<'a>(
        &'a self,
        _texture_manager: &'a TextureManager,
        _settings: &(),
    ) -> [&'a TextureView; 0] {
        []
    }

    fn get_wgpu_uniforms(&self, _settings: &Self::Settings) -> [&Buffer; 0] {
        []
    }

    fn get_draw_type(&self, _settings: &Self::Settings) -> DrawType<'_> {
        DrawType::UseInstances {
            num_vertices: 2,
            num_instances: self.len,
        }
    }
}

pub struct WireframeRoutine {
    inner: Viewport3dRoutine<WireframeLayout, NUM_BUFFERS>,
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

        self.inner.layouts.push(WireframeLayout {
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
        self.inner.add_to_graph(graph, state, &(), &[]);
    }
}
