// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;

use wgpu::{
    BlendState, ColorTargetState, ColorWrites, FragmentState, TextureFormat, VertexBufferLayout,
    VertexState,
};

#[derive(Clone, Debug)]
pub enum ShaderColorTarget {
    // The shader will write to the main viewport texture
    Viewport { use_alpha: bool },
    // The shader will write to an offscreen buffer with custom layout
    Offscreen(ColorTargetState),
}

pub struct Shader {
    pub fs_entry_point: String,
    pub vs_entry_point: String,
    pub module: wgpu::ShaderModule,
    pub color_target_descrs: Vec<ShaderColorTarget>,
    pub color_targets: Vec<Option<ColorTargetState>>,
}

impl ShaderColorTarget {
    pub fn into_wgpu(&self) -> ColorTargetState {
        match self {
            ShaderColorTarget::Viewport { use_alpha } => ColorTargetState {
                format: TextureFormat::Rgba16Float,
                blend: use_alpha.then(|| BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            },
            ShaderColorTarget::Offscreen(c) => c.clone(),
        }
    }
}

impl Shader {
    pub fn new(
        fs_entry_point: impl ToString,
        vs_entry_point: impl ToString,
        module: wgpu::ShaderModule,
        color_target_descrs: Vec<ShaderColorTarget>,
    ) -> Self {
        let color_targets = color_target_descrs
            .iter()
            .map(|d| Some(d.into_wgpu()))
            .collect();
        Self {
            fs_entry_point: fs_entry_point.to_string(),
            vs_entry_point: vs_entry_point.to_string(),
            module,
            color_target_descrs,
            color_targets,
        }
    }

    pub fn to_vertex_state<'a>(&'a self, buffers: &'a [VertexBufferLayout]) -> VertexState {
        VertexState {
            module: &self.module,
            entry_point: &self.vs_entry_point,
            buffers,
        }
    }

    pub fn get_fragment_state(&self) -> FragmentState {
        FragmentState {
            module: &self.module,
            entry_point: &self.fs_entry_point,
            targets: &self.color_targets,
        }
    }

    pub fn color_target_descriptors(&self) -> &[ShaderColorTarget] {
        &self.color_target_descrs
    }
}

pub struct ShaderManager {
    pub shaders: HashMap<String, Shader>,
}

impl ShaderManager {
    pub fn new(device: &wgpu::Device) -> Self {
        let mut shaders = HashMap::new();

        let mut context = glsl_include::Context::new();
        let context = context
            .include("utils.wgsl", include_str!("utils.wgsl"))
            .include("rend3_common.wgsl", include_str!("rend3_common.wgsl"))
            .include("rend3_vertex.wgsl", include_str!("rend3_vertex.wgsl"))
            .include("rend3_object.wgsl", include_str!("rend3_object.wgsl"))
            .include("rend3_uniforms.wgsl", include_str!("rend3_uniforms.wgsl"));

        macro_rules! def_shader {
            ($name:expr, $src:expr, opaque) => {
                def_shader!($name, $src, with_alpha, false)
            };
            ($name:expr, $src:expr, alpha_blend) => {
                def_shader!($name, $src, with_alpha, true)
            };
            ($name:expr, $src:expr, with_alpha, $use_alpha:expr) => {
                def_shader!(
                    $name,
                    $src,
                    custom,
                    vec![ShaderColorTarget::Viewport {
                        use_alpha: $use_alpha
                    }]
                )
            };
            ($name:expr, $src:expr, custom, $targets:expr) => {
                shaders.insert(
                    $name.to_string(),
                    Shader::new(
                        "fs_main",
                        "vs_main",
                        device.create_shader_module(wgpu::ShaderModuleDescriptor {
                            label: Some($name),
                            source: wgpu::ShaderSource::Wgsl(
                                context
                                    .expand(include_str!($src))
                                    .expect("Shader preprocessor")
                                    .into(),
                            ),
                        }),
                        $targets,
                    ),
                );
            };
        }

        // A bit unconventional, but shaders define their own color targets.
        // Most shaders will draw to a single Rgba16Float color buffer, either
        // in opaque mode or using alpha blending.
        def_shader!("edge_wireframe_draw", "edge_wireframe_draw.wgsl", opaque);
        def_shader!("point_cloud_draw", "point_cloud_draw.wgsl", opaque);
        def_shader!("face_draw", "face_draw.wgsl", opaque);

        // For some shaders, we use custom color targets when we have extra
        // offscreen buffers they draw to.
        def_shader!(
            "face_overlay_draw",
            "face_overlay_draw.wgsl",
            custom,
            vec![
                // First, a regular color channel, to highlight faces. The channel
                // uses transparency because it draws on top of the actual mesh.
                ShaderColorTarget::Viewport { use_alpha: true },
                // Then, the id channel, which draws to an offscreen u32 pixel
                // buffer to encode the triangle ids at each pixel.
                ShaderColorTarget::Offscreen(ColorTargetState {
                    format: wgpu::TextureFormat::R32Uint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
            ]
        );

        Self { shaders }
    }

    pub fn get(&self, shader_name: &str) -> &Shader {
        self.shaders.get(shader_name).unwrap()
    }
}
