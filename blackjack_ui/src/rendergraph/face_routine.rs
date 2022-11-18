// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use crate::{application::viewport_3d::Viewport3dSettings, prelude::r3};
use glam::Vec3;

use rend3::{
    managers::TextureManager,
    types::{Texture, TextureHandle},
};
use rend3_routine::base::{BaseRenderGraph, BaseRenderGraphIntermediateState};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use super::{
    shader_manager::ShaderManager,
    viewport_3d_routine::{DrawType, Viewport3dRoutine, RoutineLayout},
};

/// The number of matcap materials loaded in the routine. TODO: Matcaps should
/// be eventually extendible at runtime.
pub const NUM_MATCAPS: usize = 6;

/// Represents the buffers to draw a base mesh. Unlike other structures using
/// vertex pulling and instance ids to simulate indices, this buffer structure
/// uses a real index buffer. This simplifies things like smooth normals
pub struct MeshFacesLayout {
    indices: Buffer,
    positions: Buffer,
    normals: Buffer,
    matcaps: Arc<Vec<TextureHandle>>,
    num_indices: usize,
}

const BASE_MESH_NUM_BUFFERS: usize = 2;
const BASE_MESH_NUM_TEXTURES: usize = 1;
impl RoutineLayout<BASE_MESH_NUM_BUFFERS, BASE_MESH_NUM_TEXTURES> for MeshFacesLayout {
    type Settings = Viewport3dSettings;

    fn get_wgpu_buffers(&self, _settings: &Viewport3dSettings) -> [&Buffer; BASE_MESH_NUM_BUFFERS] {
        [&self.positions, &self.normals]
    }

    fn get_wgpu_textures<'a>(
        &'a self,
        texture_manager: &'a TextureManager,
        settings: &Viewport3dSettings,
    ) -> [&'a TextureView; BASE_MESH_NUM_TEXTURES] {
        [texture_manager.get_view(self.matcaps[settings.matcap % NUM_MATCAPS].get_raw())]
    }

    fn get_wgpu_uniforms<'a>(&'a self, _settings: &Self::Settings) -> [&Buffer; 0] {
        []
    }

    fn get_draw_type(&self, _settings: &Self::Settings) -> DrawType<'_> {
        DrawType::UseIndices {
            indices: &self.indices,
            num_indices: self.num_indices,
        }
    }
}

const OVERLAY_NUM_BUFFERS: usize = 2;

/// Represents the buffers to draw the face overlays, flat unshaded
/// semi-transparent triangles that are drawn over the base mesh.
pub struct FaceOverlayLayout {
    /// `3 * len` positions (as Vec3), one per triangle
    positions: Buffer,
    /// `len` colors (as Vec3), one per triangle face
    colors: Buffer,
    /// The number of faces
    len: usize,
}

impl RoutineLayout<OVERLAY_NUM_BUFFERS> for FaceOverlayLayout {
    type Settings = ();

    fn get_wgpu_buffers(&self, _settings: &Self::Settings) -> [&Buffer; OVERLAY_NUM_BUFFERS] {
        [&self.positions, &self.colors]
    }

    fn get_wgpu_textures<'a>(
        &'a self,
        _texture_manager: &'a TextureManager,
        _settings: &'a Self::Settings,
    ) -> [&'a TextureView; 0] {
        []
    }

    fn get_wgpu_uniforms<'a>(&'a self, _settings: &Self::Settings) -> [&Buffer; 0] {
        []
    }

    fn get_draw_type(&self, _settings: &Self::Settings) -> DrawType<'_> {
        DrawType::UseInstances {
            num_vertices: 3,
            num_instances: self.len,
        }
    }
}

pub struct FaceRoutine {
    matcaps: Arc<Vec<TextureHandle>>,
    base_mesh_routine: Viewport3dRoutine<MeshFacesLayout, BASE_MESH_NUM_BUFFERS, BASE_MESH_NUM_TEXTURES>,
    face_overlay_routine:
        Viewport3dRoutine<FaceOverlayLayout, OVERLAY_NUM_BUFFERS>,
}

impl FaceRoutine {
    pub fn new(
        renderer: &r3::Renderer,
        base: &BaseRenderGraph,
        shader_manager: &ShaderManager,
    ) -> Self {
        let mut matcaps = Vec::new();
        macro_rules! load_matcap {
            ($image:expr) => {
                let image = image::load_from_memory(include_bytes!(concat!(
                    "../../assets/matcap/",
                    $image,
                    ".png"
                )))
                .expect(concat!("loading texture ", $image))
                .to_rgba8();

                matcaps.push(renderer.add_texture_2d(Texture {
                    label: None,
                    data: image.to_vec(),
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    size: glam::UVec2::new(image.width(), image.height()),
                    mip_count: rend3::types::MipmapCount::Maximum,
                    mip_source: rend3::types::MipmapSource::Generated,
                }));
            };
        }

        load_matcap!("E8DEE1_B5A6AA_CCBCC1_C4BBBC");
        load_matcap!("313131_BBBBBB_878787_A3A4A4");
        load_matcap!("326666_66CBC9_C0B8AE_52B3B4");
        load_matcap!("304FB1_69A1EF_5081DF_5C8CE6");
        load_matcap!("34352A_718184_50605E_6E6761");
        load_matcap!("2E763A_78A0B7_B3D1CF_14F209");

        Self {
            matcaps: Arc::new(matcaps),
            base_mesh_routine: Viewport3dRoutine::new(
                "base mesh",
                &renderer.device,
                base,
                shader_manager.get("face_draw"),
                PrimitiveTopology::TriangleList,
                FrontFace::Cw,
                false,
            ),
            face_overlay_routine: Viewport3dRoutine::new(
                "face overlay",
                &renderer.device,
                base,
                shader_manager.get("face_overlay_draw"),
                PrimitiveTopology::TriangleList,
                FrontFace::Cw,
                true,
            ),
        }
    }

    pub fn add_base_mesh(
        &mut self,
        renderer: &r3::Renderer,
        positions: &[Vec3],
        normals: &[Vec3],
        indices: &[u32],
    ) {
        let num_indices = indices.len();
        let positions = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(positions),
            usage: BufferUsages::STORAGE,
        });
        let normals = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(normals),
            usage: BufferUsages::STORAGE,
        });
        let indices = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.base_mesh_routine.buffers.push(MeshFacesLayout {
            positions,
            normals,
            indices,
            matcaps: self.matcaps.clone(),
            num_indices,
        });
    }

    pub fn add_overlay_mesh(
        &mut self,
        renderer: &r3::Renderer,
        positions: &[Vec3],
        colors: &[Vec3],
    ) {
        let len = colors.len();
        let positions = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(positions),
            usage: BufferUsages::STORAGE,
        });
        let colors = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(colors),
            usage: BufferUsages::STORAGE,
        });

        self.face_overlay_routine.buffers.push(FaceOverlayLayout {
            positions,
            colors,
            len,
        });
    }

    pub fn clear(&mut self) {
        self.base_mesh_routine.clear();
        self.face_overlay_routine.clear();
    }

    pub fn add_to_graph<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        state: &BaseRenderGraphIntermediateState,
        settings: &'node Viewport3dSettings,
    ) {
        self.base_mesh_routine.add_to_graph(graph, state, settings);
        self.face_overlay_routine.add_to_graph(graph, state, &());
    }
}
