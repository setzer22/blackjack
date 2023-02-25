use std::num::NonZeroU64;

use glam::{IVec2, Mat4, UVec2};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupLayout, Buffer, BufferUsages, ShaderStages, TextureView,
};

use super::wgpu_utils::{BindGroupBuilder, BindGroupLayoutBuilder};
use bytemuck::{Pod, Zeroable};

pub struct ViewportRenderState {
    pub dimensions: UVec2,
    pub color_target: TextureView,
    pub depth_target: TextureView,
    /// Contains a ViewportUniforms
    pub viewport_uniforms_bg: BindGroup,
}

#[derive(Pod, Zeroable, Copy, Clone, Debug)]
#[repr(C)]
pub struct ViewportUniforms {
    pub camera_view: glam::Mat4,
    pub camera_projection: glam::Mat4,
}

impl ViewportRenderState {
    pub fn new(
        device: &wgpu::Device,
        dimensions: UVec2,
        color_target: TextureView,
        depth_target: TextureView,
    ) -> Self {
        let mut bgb = BindGroupBuilder::new();
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&ViewportUniforms {
                camera_view: Mat4::IDENTITY,
                camera_projection: Mat4::IDENTITY,
            }),
            usage: BufferUsages::UNIFORM,
        });
        bgb.append_buffer(&buffer);

        Self {
            dimensions,
            color_target,
            depth_target,
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
