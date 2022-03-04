use super::common;
use crate::prelude::r3;
use glam::Vec3;
use rend3::{
    graph::DataHandle,
    util::bind_merge::{BindGroupBuilder, BindGroupLayoutBuilder},
};
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

pub struct WireframeRoutine {
    bgl: BindGroupLayout,
    pipeline: RenderPipeline,
    wireframe_buffers: Vec<WireframeBuffer>,
}

impl WireframeRoutine {
    pub fn new(device: &Device, base: &BaseRenderGraph, shader_manager: &ShaderManager) -> Self {
        let bgl = BindGroupLayoutBuilder::new()
            // Binding 0 is the buffer with line data, with 2*N points, each two
            // points making a line segment
            .append(
                ShaderStages::VERTEX,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                None,
            )
            // Binding 1 is the buffer with color data, with N colors
            .append(
                ShaderStages::VERTEX,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                None,
            )
            .build(device, Some("wireframe routine bgl"));

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&base.interfaces.forward_uniform_bgl, &bgl],
            push_constant_ranges: &[],
        });

        let shader = shader_manager.get("edge_wireframe_draw");

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("wireframe routine pipeline"),
            layout: Some(&pipeline_layout),
            vertex: shader.to_vertex_state(&[]),
            primitive: common::primitive_state(PrimitiveTopology::LineList),
            depth_stencil: Some(common::depth_stencil(true)),
            multisample: MultisampleState::default(),
            fragment: Some(shader.to_fragment_state()),
            multiview: None,
        });

        Self {
            pipeline,
            bgl,
            wireframe_buffers: Vec::new(),
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

        self.wireframe_buffers.push(WireframeBuffer {
            len,
            line_positions,
            colors,
        });
    }

    pub fn clear(&mut self) {
        // Wgpu will deallocate resources when `Drop` is called for the buffers.
        self.wireframe_buffers.clear()
    }

    fn create_bind_groups<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        out_bgs: DataHandle<Vec<BindGroup>>,
    ) {
        let mut builder = graph.add_node("Wireframe: create bind groups");
        let pt_handle = builder.passthrough_ref(self);
        let out_bgs = builder.add_data_output(out_bgs);

        builder.build(
            move |pt, renderer, _encoder_or_pass, _temps, _ready, graph_data| {
                let this = pt.get(pt_handle);
                graph_data.set_data(
                    out_bgs,
                    Some(
                        self.wireframe_buffers
                            .iter()
                            .map(|buffer| {
                                BindGroupBuilder::new()
                                    .append_buffer(&buffer.line_positions)
                                    .append_buffer(&buffer.colors)
                                    .build(&renderer.device, None, &this.bgl)
                            })
                            .collect(),
                    ),
                );
            },
        )
    }

    fn draw_wireframe<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        state: &BaseRenderGraphIntermediateState,
        in_bgs: DataHandle<Vec<BindGroup>>,
    ) {
        let mut builder = graph.add_node("Wireframe: draw");
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
                for (buffer, bg) in this.wireframe_buffers.iter().zip(in_bgs.iter()) {
                    pass.set_bind_group(1, bg, &[]);
                    pass.draw(0..2, 0..buffer.len as u32);
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
        self.draw_wireframe(graph, state, bgs);
    }
}
