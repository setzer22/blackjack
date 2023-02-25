// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::num::{NonZeroU32, NonZeroU64, NonZeroU8};

use wgpu::*;

pub fn primitive_state(
    topology: wgpu::PrimitiveTopology,
    front_face: wgpu::FrontFace,
) -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        topology,
        strip_index_format: None,
        front_face,
        cull_mode: Some(wgpu::Face::Back),
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::Fill,
        conservative: false,
    }
}

pub fn depth_stencil(depth_write: bool) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: depth_write,
        depth_compare: wgpu::CompareFunction::GreaterEqual,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

pub fn create_render_texture(
    device: &Device,
    label: &str,
    size: glam::UVec2,
    format: Option<TextureFormat>,
) -> (Texture, TextureView) {
    let format = format.unwrap_or(wgpu::TextureFormat::Rgba16Float);

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        label: Some(label),
    });

    let view = texture.create_view(&TextureViewDescriptor {
        label: Some(label),
        format: Some(format),
        dimension: Some(wgpu::TextureViewDimension::D2),
        aspect: TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    });

    (texture, view)
}

pub fn create_sampler(
    device: &Device,
    filter: FilterMode,
    compare: Option<CompareFunction>,
) -> Sampler {
    device.create_sampler(&SamplerDescriptor {
        label: Some("linear"),
        address_mode_u: AddressMode::Repeat,
        address_mode_v: AddressMode::Repeat,
        address_mode_w: AddressMode::Repeat,
        mag_filter: filter,
        min_filter: filter,
        mipmap_filter: filter,
        lod_min_clamp: 0.0,
        lod_max_clamp: 100.0,
        compare,
        anisotropy_clamp: NonZeroU8::new(16),
        border_color: None,
    })
}

/// Builder for BindGroupLayouts.
pub struct BindGroupLayoutBuilder {
    bgl_entries: Vec<BindGroupLayoutEntry>,
}
impl BindGroupLayoutBuilder {
    pub fn new() -> Self {
        Self {
            bgl_entries: Vec::with_capacity(16),
        }
    }

    pub fn append(
        &mut self,
        visibility: ShaderStages,
        ty: BindingType,
        count: Option<NonZeroU32>,
    ) -> &mut Self {
        let binding = self.bgl_entries.len() as u32;
        self.bgl_entries.push(BindGroupLayoutEntry {
            binding,
            visibility,
            ty,
            count,
        });
        self
    }

    pub fn build(&self, device: &Device, label: Option<&str>) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label,
            entries: &self.bgl_entries,
        })
    }
}

impl Default for BindGroupLayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for BindGroups.
pub struct BindGroupBuilder<'a> {
    bg_entries: Vec<BindGroupEntry<'a>>,
}
impl<'a> BindGroupBuilder<'a> {
    pub fn new() -> Self {
        Self {
            bg_entries: Vec::with_capacity(16),
        }
    }

    pub fn append(&mut self, resource: BindingResource<'a>) -> &mut Self {
        let index = self.bg_entries.len();
        self.bg_entries.push(BindGroupEntry {
            binding: index as u32,
            resource,
        });
        self
    }

    pub fn append_buffer(&mut self, buffer: &'a Buffer) -> &mut Self {
        self.append(buffer.as_entire_binding());
        self
    }

    pub fn append_buffer_with_size(&mut self, buffer: &'a Buffer, size: u64) -> &mut Self {
        self.append(BindingResource::Buffer(BufferBinding {
            buffer,
            offset: 0,
            size: NonZeroU64::new(size),
        }));
        self
    }

    pub fn append_sampler(&mut self, sampler: &'a Sampler) -> &mut Self {
        self.append(BindingResource::Sampler(sampler));
        self
    }

    pub fn append_texture_view(&mut self, texture_view: &'a TextureView) -> &mut Self {
        self.append(BindingResource::TextureView(texture_view));
        self
    }

    pub fn append_texture_view_array(
        &mut self,
        texture_view_array: &'a [&'a TextureView],
    ) -> &mut Self {
        self.append(BindingResource::TextureViewArray(texture_view_array));
        self
    }

    pub fn build(&self, device: &Device, label: Option<&str>, bgl: &BindGroupLayout) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label,
            layout: bgl,
            entries: &self.bg_entries,
        })
    }
}

impl<'a> Default for BindGroupBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}
