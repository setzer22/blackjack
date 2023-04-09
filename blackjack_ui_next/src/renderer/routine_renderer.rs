// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::blackjack_theme::pallette;

use super::{
    render_state::ViewportRenderState,
    shader_manager::{Shader, ShaderColorTarget},
    texture_manager::TextureManager,
    wgpu_utils::{self, BindGroupBuilder, BindGroupLayoutBuilder},
};
use guee::extension_traits::Color32Ext;
use wgpu::*;

pub enum DrawType<'a> {
    /// Uses vertex pulling with an index buffer. The vertex id is used to index
    /// the storage buffers.
    UseIndices {
        indices: &'a Buffer,
        num_indices: usize,
    },
    /// Uses vertex pulling without an index buffer, will draw instances of
    /// `num_vertices` and the instance id will is used to index the storage
    UseInstances {
        num_vertices: usize,
        num_instances: usize,
    },
}

/// Generic trait to set different parameters of the viewport display.
///
/// Will generate a layout with given storage buffers, textures and uniform
/// buffers. Any of the three could be left as empty and will be generated in
/// the following order: (storages, textures, uniforms). All bindings will be
/// added to bind group 1, since bind group 0 is already used by rend3.
pub trait RoutineLayout {
    type Settings;

    /// Returns one wgpu buffer for each of the `NUM_BUFFERS` buffers
    fn get_wgpu_buffers(&self, settings: &Self::Settings) -> Vec<&Buffer>;

    /// Returns one wgpu buffer for each of the `NUM_TEXTURES` buffers
    fn get_wgpu_textures<'a>(
        &self,
        texture_manager: &'a TextureManager,
        settings: &Self::Settings,
    ) -> Vec<&'a TextureView>;

    /// Returns one wgpu uniform for eah of the `NUM_UNIFORMS` buffers
    fn get_wgpu_uniforms(&self, settings: &Self::Settings) -> Vec<&Buffer>;

    /// Returns the draw type that should be used to draw this routine. Either
    /// spawn a fixed number of primitives, or use an index buffer.
    fn get_draw_type(&self, settings: &Self::Settings) -> DrawType<'_>;

    /// Returns the number of buffers that are used by this routine. The
    /// get_wgpu_buffers method shuld return the same number of buffers.
    fn num_buffers() -> usize {
        // Default value
        0
    }

    /// Returns the number of textures that are used by this routine. The
    /// get_wgpu_textures method shuld return the same number of buffers.
    fn num_textures() -> usize {
        // Default value
        0
    }

    /// Returns the number of uniforms that are used by this routine. The
    /// get_wgpu_uniforms method shuld return the same number of buffers.
    fn num_uniforms() -> usize {
        // Default value
        0
    }
}

#[derive(Clone, Copy)]
pub enum MultisampleConfig {
    One,
    Four,
}

impl MultisampleConfig {
    pub fn to_u32(self) -> u32 {
        match self {
            MultisampleConfig::One => 1,
            MultisampleConfig::Four => 4,
        }
    }

    pub fn to_multisample_state(self) -> MultisampleState {
        match self {
            MultisampleConfig::One => MultisampleState::default(),
            MultisampleConfig::Four => MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        }
    }
}

/// A structure holding the configuration to render a routine.
pub struct RenderCommand<'a, Settings> {
    /// The [`TextureManager`]
    pub texture_manager: &'a TextureManager,
    /// The [`ViewportRenderState`]
    pub render_state: &'a ViewportRenderState,
    /// The settings. Should match the Layout.
    pub settings: &'a Settings,
    /// For each ShaderColorTarget::Offscreen in the provided shader, one
    /// texture view handle matching its configuration.
    pub offscreen_targets: &'a [&'a TextureView],
    /// If provided, the depth texture view to use for this render pass.
    pub override_depth: Option<&'a TextureView>,
    /// If true, the provided targets will be cleared before the render pass.
    pub clear_buffer: bool,
}

impl<'a, Settings> RenderCommand<'a, Settings> {
    pub fn new(
        texture_manager: &'a TextureManager,
        render_state: &'a ViewportRenderState,
        settings: &'a Settings,
    ) -> Self {
        Self {
            texture_manager,
            render_state,
            settings,
            offscreen_targets: &[],
            override_depth: None,
            clear_buffer: false,
        }
    }

    pub fn offscren_targets(&mut self, offscreen_targets: &'a [&'a TextureView]) -> &mut Self {
        self.offscreen_targets = offscreen_targets;
        self
    }

    pub fn override_depth(&mut self, override_depth: Option<&'a TextureView>) -> &mut Self {
        self.override_depth = override_depth;
        self
    }

    pub fn clear_buffer(&mut self, clear_buffer: bool) -> &mut Self {
        self.clear_buffer = clear_buffer;
        self
    }
}

pub struct RoutineRenderer<Layout: RoutineLayout> {
    name: String,
    bgl: BindGroupLayout,
    pipeline: RenderPipeline,
    pub layouts: Vec<Layout>,
    pub color_target_descrs: Vec<ShaderColorTarget>,
    pub multisample: MultisampleConfig,
}

impl<Layout: RoutineLayout + 'static> RoutineRenderer<Layout> {
    pub fn new(
        name: &str,
        device: &Device,
        shader: &Shader,
        topology: PrimitiveTopology,
        front_face: FrontFace,
        multisample: MultisampleConfig,
    ) -> Self {
        let bgl = {
            let mut builder = BindGroupLayoutBuilder::new();
            for _ in 0..Layout::num_buffers() {
                builder.append(
                    ShaderStages::VERTEX_FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    None,
                );
            }
            for _ in 0..Layout::num_textures() {
                builder.append(
                    ShaderStages::VERTEX_FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    None,
                );
            }
            for _ in 0..Layout::num_uniforms() {
                builder.append(
                    ShaderStages::VERTEX_FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
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
            bind_group_layouts: &[&ViewportRenderState::viewport_uniforms_layout(device), &bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&format!("{name} render pipeline")),
            layout: Some(&pipeline_layout),
            vertex: shader.to_vertex_state(&[]),
            primitive: wgpu_utils::primitive_state(topology, front_face),
            depth_stencil: Some(wgpu_utils::depth_stencil(true)),
            multisample: multisample.to_multisample_state(),
            fragment: Some(shader.get_fragment_state()),
            multiview: None,
        });

        Self {
            name: name.into(),
            pipeline,
            bgl,
            layouts: Vec::new(),
            color_target_descrs: shader.color_target_descrs.clone(),
            multisample,
        }
    }

    pub fn clear(&mut self) {
        // Wgpu will deallocate resources when `Drop` is called for the buffers.
        self.layouts.clear()
    }

    pub fn create_bind_groups(
        &self,
        device: &Device,
        texture_manager: &TextureManager,
        settings: &Layout::Settings,
    ) -> Vec<BindGroup> {
        self.layouts
            .iter()
            .map(|buffer| {
                let mut builder = BindGroupBuilder::new();
                for buffer in buffer.get_wgpu_buffers(settings) {
                    builder.append_buffer(buffer);
                }
                for texture in buffer.get_wgpu_textures(texture_manager, settings) {
                    builder.append_texture_view(texture);
                }
                for uniform in buffer.get_wgpu_uniforms(settings) {
                    builder.append_buffer(uniform);
                }
                builder.build(device, None, &self.bgl)
            })
            .collect()
    }

    pub fn render(
        &self,
        device: &Device,
        encoder: &mut CommandEncoder,
        command: &mut RenderCommand<'_, Layout::Settings>,
    ) {
        let mut color_attachments = vec![];
        let mut offscreen_targets = command.offscreen_targets.iter();
        for d in &self.color_target_descrs {
            let clear_color = pallette().background_dark;
            let clear_color = wgpu::Color {
                r: clear_color.red_f().powf(2.2) as f64,
                g: clear_color.green_f().powf(2.2) as f64,
                b: clear_color.blue_f().powf(2.2) as f64,
                a: clear_color.alpha_f().powf(2.2) as f64,
            };
            let ops = Operations {
                load: if command.clear_buffer {
                    LoadOp::Clear(clear_color)
                } else {
                    LoadOp::Load
                },
                store: true,
            };
            match d {
                ShaderColorTarget::Viewport { use_alpha: _ } => {
                    color_attachments.push(Some(RenderPassColorAttachment {
                        view: &command.render_state.color_target,
                        resolve_target: command.render_state.color_resolve_target.as_ref(),
                        ops,
                    }));
                }
                ShaderColorTarget::Offscreen(_) => {
                    color_attachments.push(Some(RenderPassColorAttachment {
                        view: offscreen_targets
                            .next()
                            .expect("Not enough offscreen buffer handles"),
                        resolve_target: None,
                        ops,
                    }));
                }
            }
        }

        let bind_groups =
            self.create_bind_groups(device, command.texture_manager, command.settings);
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(&format!("Blackjack Viewport3d RenderPass: {}", self.name)),
            color_attachments: &color_attachments,
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: command
                    .override_depth
                    .unwrap_or(&command.render_state.color_depth_target),
                depth_ops: Some(Operations {
                    load: if command.clear_buffer {
                        LoadOp::Clear(0.0)
                    } else {
                        LoadOp::Load
                    },
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &command.render_state.viewport_uniforms_bg, &[]);
        for (buffer, bg) in self.layouts.iter().zip(bind_groups.iter()) {
            pass.set_bind_group(1, bg, &[]);

            match buffer.get_draw_type(command.settings) {
                DrawType::UseIndices {
                    indices,
                    num_indices,
                } => {
                    pass.set_index_buffer(indices.slice(..), IndexFormat::Uint32);
                    pass.draw_indexed(0..num_indices as u32, 0, 0..1);
                }
                DrawType::UseInstances {
                    num_vertices,
                    num_instances,
                } => {
                    pass.draw(0..num_vertices as u32, 0..num_instances as u32);
                }
            }
        }
    }
}
