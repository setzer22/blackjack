// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use glam::Vec3;

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use super::{
    render_state::ViewportRenderState,
    routine_renderer::{
        DrawType, MultisampleConfig, RenderCommand, RoutineLayout, RoutineRenderer,
    },
    shader_manager::ShaderManager,
    texture_manager::TextureManager,
};

pub struct PointCloudLayout {
    buffer: Buffer,
    len: usize,
}

impl RoutineLayout for PointCloudLayout {
    type Settings = ();
    fn get_wgpu_buffers(&self, _settings: &()) -> Vec<&Buffer> {
        vec![&self.buffer]
    }

    fn get_wgpu_textures<'a>(
        &self,
        _texture_manager: &'a TextureManager,
        _settings: &(),
    ) -> Vec<&'a TextureView> {
        vec![]
    }

    fn get_wgpu_uniforms(&self, _settings: &Self::Settings) -> Vec<&Buffer> {
        vec![]
    }

    fn get_draw_type(&self, _settings: &Self::Settings) -> DrawType<'_> {
        DrawType::UseInstances {
            num_vertices: 6,
            num_instances: self.len,
        }
    }

    fn num_buffers() -> usize {
        1
    }
}

pub struct PointCloudRoutine {
    inner: RoutineRenderer<PointCloudLayout>,
}

impl PointCloudRoutine {
    pub fn new(
        device: &Device,
        shader_manager: &ShaderManager,
        multisample: MultisampleConfig,
    ) -> Self {
        Self {
            inner: RoutineRenderer::new(
                "point cloud",
                device,
                shader_manager.get("point_cloud_draw"),
                PrimitiveTopology::TriangleList,
                FrontFace::Ccw,
                multisample,
            ),
        }
    }

    pub fn add_point_cloud(&mut self, device: &Device, points: &[Vec3]) {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(points),
            usage: BufferUsages::STORAGE,
        });
        self.inner.layouts.push(PointCloudLayout {
            buffer,
            len: points.len(),
        });
    }

    pub fn clear(&mut self) {
        self.inner.clear()
    }

    pub fn render(
        &self,
        device: &Device,
        encoder: &mut CommandEncoder,
        texture_manager: &TextureManager,
        render_state: &ViewportRenderState,
        clear_buffer: bool,
    ) {
        self.inner.render(
            device,
            encoder,
            RenderCommand::new(texture_manager, render_state, &()).clear_buffer(clear_buffer),
        )
    }
}
