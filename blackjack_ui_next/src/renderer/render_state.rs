use std::num::NonZeroU64;

use glam::{IVec2, Mat4, UVec2};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupLayout, Buffer, BufferUsages, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, Sampler, SamplerDescriptor, ShaderStages, TextureView,
};

use super::wgpu_utils::{self, BindGroupBuilder, BindGroupLayoutBuilder};
use bytemuck::{Pod, Zeroable};

pub struct ViewportRenderState {
    pub dimensions: UVec2,
    pub color_target: TextureView,
    pub color_resolve_target: Option<TextureView>,
    pub depth_target: TextureView,
    /// Contains a ViewportUniforms
    pub viewport_uniforms_bg: BindGroup,
}

/// NOTE: Must match definitions in uniforms.wgsl
#[derive(Pod, Zeroable, Copy, Clone, Debug)]
#[repr(C)]
pub struct ViewportUniforms {
    pub view: glam::Mat4,
    pub proj: glam::Mat4,
    pub view_proj: glam::Mat4,
    pub inv_view: glam::Mat4,
    pub inv_proj: glam::Mat4,
    pub resolution: UVec2,
    pub _padding: [UVec2; 3],
}

impl ViewportUniforms {
    pub fn new(view: glam::Mat4, proj: glam::Mat4, resolution: UVec2) -> Self {
        Self {
            view,
            proj,
            view_proj: proj * view,
            inv_view: view.inverse(),
            inv_proj: proj.inverse(),
            resolution,
            _padding: Default::default(),
        }
    }
}

impl ViewportRenderState {
    pub fn new(
        device: &wgpu::Device,
        dimensions: UVec2,
        color_target: TextureView,
        depth_target: TextureView,
        color_resolve_target: Option<TextureView>,
        uniforms: ViewportUniforms,
    ) -> Self {
        let mut bgb = BindGroupBuilder::new();
        let sampler = wgpu_utils::create_sampler(device, wgpu::FilterMode::Linear, None);
        bgb.append_sampler(&sampler);
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&uniforms),
            usage: BufferUsages::UNIFORM,
        });
        bgb.append_buffer(&buffer);

        Self {
            dimensions,
            color_target,
            depth_target,
            color_resolve_target,
            viewport_uniforms_bg: bgb.build(
                device,
                Some("Blackjack Viewport Uniforms"),
                &Self::viewport_uniforms_layout(device),
            ),
        }
    }

    pub fn viewport_uniforms_layout(device: &wgpu::Device) -> BindGroupLayout {
        let mut bglb = BindGroupLayoutBuilder::new();
        bglb.append(
            ShaderStages::VERTEX_FRAGMENT,
            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            None,
        );
        bglb.append(
            ShaderStages::VERTEX_FRAGMENT,
            wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: NonZeroU64::new(std::mem::size_of::<ViewportUniforms>() as u64),
            },
            None,
        );
        bglb.build(device, Some("Blackjack3d Viewport Uniforms"))
    }
}
