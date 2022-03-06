use super::{common, shader_manager::Shader};
use crate::prelude::r3;
use rend3::{
    graph::DataHandle,
    util::bind_merge::{BindGroupBuilder, BindGroupLayoutBuilder},
};
use rend3_routine::base::{BaseRenderGraph, BaseRenderGraphIntermediateState};
use wgpu::*;

/// Stores a wgpu buffer containing the edges of a wireframe
pub trait ViewportBuffers {
    const NUM_BUFFERS: usize;
    /// Returns one wgpu buffer for each of the `NUM_BUFFERS` buffers
    fn get_wgpu_buffers(&self) -> Vec<&Buffer>;
    /// Returns the number of vertex and instances to issue for each draw call
    fn vertex_instance_counts(&self) -> (u32, u32);
}

pub struct Viewport3dRoutine<Buffers: ViewportBuffers> {
    bgl: BindGroupLayout,
    pipeline: RenderPipeline,
    pub buffers: Vec<Buffers>,
}

impl<Buffers: ViewportBuffers + 'static> Viewport3dRoutine<Buffers> {
    pub fn new(
        name: &str,
        device: &Device,
        base: &BaseRenderGraph,
        shader: &Shader,
        topology: PrimitiveTopology,
        front_face: FrontFace,
    ) -> Self {
        let bgl = {
            let mut builder = BindGroupLayoutBuilder::new();
            for _ in 0..Buffers::NUM_BUFFERS {
                builder.append(
                    ShaderStages::VERTEX,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    None,
                );
            }
            builder.build(device, Some(&format!("{name} bgl")))
        };

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&base.interfaces.forward_uniform_bgl, &bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&format!("{name} render pipeline")),
            layout: Some(&pipeline_layout),
            vertex: shader.to_vertex_state(&[]),
            primitive: common::primitive_state(topology, front_face),
            depth_stencil: Some(common::depth_stencil(true)),
            multisample: MultisampleState::default(),
            fragment: Some(shader.to_fragment_state()),
            multiview: None,
        });

        Self {
            pipeline,
            bgl,
            buffers: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        // Wgpu will deallocate resources when `Drop` is called for the buffers.
        self.buffers.clear()
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
                        self.buffers
                            .iter()
                            .map(|buffer| {
                                let mut builder = BindGroupBuilder::new();
                                for buffer in buffer.get_wgpu_buffers() {
                                    builder.append_buffer(buffer);
                                }
                                builder.build(&renderer.device, None, &this.bgl)
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
                for (buffer, bg) in this.buffers.iter().zip(in_bgs.iter()) {
                    let (vertex_count, instance_count) = buffer.vertex_instance_counts();
                    pass.set_bind_group(1, bg, &[]);
                    pass.draw(0..vertex_count, 0..instance_count as u32);
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
