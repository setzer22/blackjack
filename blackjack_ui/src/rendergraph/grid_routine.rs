// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::prelude::*;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupLayout, Color, Device, RenderPipeline,
};

pub struct GridRoutine {
    pipeline: RenderPipeline,
    bgl: BindGroupLayout,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct GridRoutineUniform {
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub inv_view: [[f32; 4]; 4],
    pub inv_proj: [[f32; 4]; 4],
}

impl GridRoutine {
    pub fn new(device: &Device) -> Self {
        use wgpu::*;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("grid_shader.wgsl").into()),
        });

        let _uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Grid uniform buffer"),
            size: std::mem::size_of::<GridRoutineUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::MAP_WRITE,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Grid BGL"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Grid pipeline layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Grid Pipeline"),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Self { pipeline, bgl }
    }

    fn grid_pass<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        color: r3::RenderTargetHandle,
        depth: r3::RenderTargetHandle,
        resolve: Option<r3::RenderTargetHandle>,
        grid_uniform_bg: r3::DataHandle<BindGroup>,
    ) {
        let mut builder = graph.add_node("Infinite Grid");
        let color_handle = builder.add_render_target_output(color);
        let resolve = builder.add_optional_render_target_output(resolve);
        let depth_handle = builder.add_render_target_output(depth);

        let rpass_handle = builder.add_renderpass(r3::RenderPassTargets {
            targets: vec![r3::RenderPassTarget {
                color: color_handle,
                clear: Color::BLACK,
                resolve,
            }],
            depth_stencil: Some(r3::RenderPassDepthTarget {
                target: r3::DepthHandle::RenderTarget(depth_handle),
                depth_clear: Some(0.0),
                stencil_clear: None,
            }),
        });

        let grid_uniform_handle = builder.add_data_input(grid_uniform_bg);
        let pt_handle = builder.passthrough_ref(self);

        builder.build(
            move |pt, _renderer, encoder_or_pass, temps, _ready, graph_data| {
                let this = pt.get(pt_handle);
                let rpass = encoder_or_pass.get_rpass(rpass_handle);
                let grid_uniform_bg = graph_data.get_data(temps, grid_uniform_handle).unwrap();

                rpass.set_bind_group(0, grid_uniform_bg, &[]);
                rpass.set_pipeline(&this.pipeline);
                rpass.draw(0..6, 0..1);
            },
        );
    }

    fn create_bind_groups<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        grid_uniform_bg: r3::DataHandle<BindGroup>,
    ) {
        use wgpu::*;
        let mut builder = graph.add_node("build grid uniforms");
        let output_handle = builder.add_data_output(grid_uniform_bg);
        let pt_handle = builder.passthrough_ref(self);
        builder.build(
            move |pt, renderer, _encoder_or_pass, _temps, _ready, graph_data| {
                let this = pt.get(pt_handle);

                let camera_manager = graph_data.camera_manager;
                let cam_data = GridRoutineUniform {
                    view: camera_manager.view().to_cols_array_2d(),
                    proj: camera_manager.proj().to_cols_array_2d(),
                    inv_view: camera_manager.view().inverse().to_cols_array_2d(),
                    inv_proj: camera_manager.proj().inverse().to_cols_array_2d(),
                };

                let buffer = renderer.device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&[cam_data]),
                    usage: BufferUsages::UNIFORM,
                });

                let bind_group = renderer.device.create_bind_group(&BindGroupDescriptor {
                    label: Some("Grid BindGroup"),
                    layout: &this.bgl,
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                });

                graph_data.set_data(output_handle, Some(bind_group));
            },
        );
    }

    pub fn add_to_graph<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        state: &r3::BaseRenderGraphIntermediateState,
    ) {
        let grid_uniform_bg = graph.add_data::<BindGroup>();
        self.create_bind_groups(graph, grid_uniform_bg);
        self.grid_pass(
            graph,
            state.color,
            state.depth,
            state.resolve,
            grid_uniform_bg,
        );
    }
}
