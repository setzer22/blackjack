// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use wgpu::*;

use crate::renderer::wgpu_utils;

use super::{
    render_state::ViewportRenderState, routine_renderer::MultisampleConfig,
    shader_manager::ShaderManager,
};

pub struct GridRoutine {
    pipeline: RenderPipeline,
}

impl GridRoutine {
    pub fn new(
        device: &Device,
        shader_manager: &ShaderManager,
        multisample_config: MultisampleConfig,
    ) -> Self {
        use wgpu::*;
        let shader = shader_manager.get("grid_shader");

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Grid pipeline layout"),
            bind_group_layouts: &[&ViewportRenderState::viewport_uniforms_layout(device)],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Grid Pipeline"),
            layout: Some(&layout),
            vertex: shader.to_vertex_state(&[]),
            primitive: wgpu_utils::primitive_state(PrimitiveTopology::TriangleList, FrontFace::Ccw),
            depth_stencil: Some(wgpu_utils::depth_stencil(true)),
            multisample: multisample_config.to_multisample_state(),
            fragment: Some(shader.get_fragment_state()),
            multiview: None,
        });

        Self { pipeline }
    }

    pub fn render(&self, encoder: &mut CommandEncoder, render_state: &ViewportRenderState) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Blackjack Grid"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &render_state.color_target,
                resolve_target: render_state.color_resolve_target.as_ref(),
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &render_state.color_depth_target,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        pass.set_bind_group(0, &render_state.viewport_uniforms_bg, &[]);
        pass.set_pipeline(&self.pipeline);
        pass.draw(0..6, 0..1);
    }
}
