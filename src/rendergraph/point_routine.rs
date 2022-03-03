use std::num::NonZeroU64;

use crate::prelude::r3;
use glam::Vec3;
use rend3::{
    graph::{DataHandle, RenderTargetDescriptor},
    util::bind_merge::{BindGroupBuilder, BindGroupLayoutBuilder},
};
use rend3_routine::base::{BaseRenderGraph, BaseRenderGraphIntermediateState};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use super::shader_manager::ShaderManager;

pub struct PointCloudBuffer {
    buffer: Buffer,
    len: usize,
}

pub struct PointCloudRoutine {
    bgl: BindGroupLayout,
    pipeline: RenderPipeline,
    point_cloud_buffers: Vec<PointCloudBuffer>,
}

const PRIMITIVE_STATE: PrimitiveState = PrimitiveState {
    topology: PrimitiveTopology::TriangleList,
    strip_index_format: None,
    front_face: FrontFace::Ccw,
    cull_mode: Some(Face::Back),
    unclipped_depth: false,
    polygon_mode: PolygonMode::Fill,
    conservative: false,
};

impl PointCloudRoutine {
    pub fn new(device: &Device, base: &BaseRenderGraph, shader_manager: &ShaderManager) -> Self {
        let bgl = BindGroupLayoutBuilder::new()
            // Binding 0 is the buffer with all the points
            .append(
                ShaderStages::VERTEX,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                None,
            )
            .build(device, Some("point routine bgl"));

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&base.interfaces.forward_uniform_bgl, &bgl],
            push_constant_ranges: &[],
        });

        let shader = shader_manager.get("point_cloud_draw");

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("point routine pipeline"),
            layout: Some(&pipeline_layout),
            vertex: shader.to_vertex_state(&[]),
            primitive: PRIMITIVE_STATE,
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            fragment: Some(shader.to_fragment_state(&[ColorTargetState {
                format: TextureFormat::Rgba16Float,
                blend: None,
                write_mask: ColorWrites::all(),
            }])),
            multiview: None,
        });

        Self {
            pipeline,
            bgl,
            point_cloud_buffers: Vec::new(),
        }
    }

    pub fn add_point_cloud(&mut self, device: &Device, points: &[Vec3]) {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(points),
            usage: BufferUsages::STORAGE,
        });
        self.point_cloud_buffers.push(PointCloudBuffer {
            buffer,
            len: points.len(),
        });
    }

    pub fn clear_point_clouds(&mut self) {
        // Wgpu will deallocate resources when `Drop` is called for the buffers.
        self.point_cloud_buffers.clear()
    }

    pub fn create_bind_groups<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        out_bgs: DataHandle<Vec<BindGroup>>,
    ) {
        let mut builder = graph.add_node("Point cloud: create bind groups");
        let pt_handle = builder.passthrough_ref(self);
        let out_bgs = builder.add_data_output(out_bgs);

        builder.build(
            move |pt, renderer, _encoder_or_pass, _temps, _ready, graph_data| {
                let this = pt.get(pt_handle);
                graph_data.set_data(
                    out_bgs,
                    Some(
                        self.point_cloud_buffers
                            .iter()
                            .map(|buffer| {
                                BindGroupBuilder::new().append_buffer(&buffer.buffer).build(
                                    &renderer.device,
                                    None,
                                    &this.bgl,
                                )
                            })
                            .collect(),
                    ),
                );
            },
        )
    }

    pub fn draw_point_cloud<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        state: &BaseRenderGraphIntermediateState,
        in_bgs: DataHandle<Vec<BindGroup>>,
    ) {
        let mut builder = graph.add_node("Point cloud: draw points");
        let color = builder.add_render_target_output(state.color);
        let depth = builder.add_render_target_output(state.depth);
        let in_bgs = builder.add_data_input(in_bgs);
        let resolve = builder.add_optional_render_target_output(state.resolve);
        let pt_handle = builder.passthrough_ref(self);
        let forward_uniform_bg = builder.add_data_input(state.forward_uniform_bg);


        let rpass_handle = builder.add_renderpass(r3::RenderPassTargets {
            targets: vec![r3::RenderPassTarget {
                color,
                clear: Color::BLACK,
                resolve,
            }],
            depth_stencil: Some(r3::RenderPassDepthTarget {
                target: r3::DepthHandle::RenderTarget(depth),
                depth_clear: Some(0.0),
                stencil_clear: None,
            }),
        });

        builder.build(
            move |pt, _renderer, encoder_or_pass, temps, _ready, graph_data| {
                let this = pt.get(pt_handle);
                let pass = encoder_or_pass.get_rpass(rpass_handle);

                let in_bgs = graph_data.get_data(temps, in_bgs).unwrap();
                let forward_uniform_bg = graph_data.get_data(temps, forward_uniform_bg).unwrap();

                pass.set_pipeline(&this.pipeline);

                pass.set_bind_group(0, forward_uniform_bg, &[]);
                for (buffer, bg) in this.point_cloud_buffers.iter().zip(in_bgs.iter()) {
                    pass.set_bind_group(1, bg, &[]);
                    pass.draw(0..6, 0..dbg!(buffer.len) as u32);
                }
            },
        );
    }

    pub fn add_to_graph<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        state: &BaseRenderGraphIntermediateState,
    ) {
        let bgs = graph.add_data();
        self.create_bind_groups(graph, bgs);
        self.draw_point_cloud(graph, state, bgs);
    }
}
