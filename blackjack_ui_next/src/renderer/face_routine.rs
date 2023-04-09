// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use glam::{Vec3, Vec4};

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::viewport_3d::Viewport3dSettings;

use super::{
    render_state::ViewportRenderState,
    routine_renderer::{DrawType, MultisampleConfig, RoutineLayout, RoutineRenderer},
    shader_manager::ShaderManager,
    texture_manager::TextureManager,
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
    matcaps: Arc<Vec<String>>,
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
        [texture_manager
            .get_texture_view(&self.matcaps[settings.matcap % NUM_MATCAPS])
            .as_ref()
            .unwrap()]
    }

    fn get_wgpu_uniforms(&self, _settings: &Self::Settings) -> [&Buffer; 0] {
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
const OVERLAY_NUM_UNIFORMS: usize = 0;

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

impl RoutineLayout<OVERLAY_NUM_BUFFERS, 0, OVERLAY_NUM_UNIFORMS> for FaceOverlayLayout {
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

    fn get_wgpu_uniforms(&self, _settings: &Self::Settings) -> [&Buffer; OVERLAY_NUM_UNIFORMS] {
        []
    }

    fn get_draw_type(&self, _settings: &Self::Settings) -> DrawType<'_> {
        DrawType::UseInstances {
            num_vertices: 3,
            num_instances: self.len,
        }
    }
}

const ID_NUM_BUFFERS: usize = 2;
const ID_NUM_UNIFORMS: usize = 1;

/// Represents the buffers to draw the face ids, used to perform mouse picking.
pub struct FaceIdLayout {
    /// `3 * len` positions (as Vec3), one per triangle
    positions: Buffer,
    /// `len` face ids, one per triangle. Multilpe triangles may share the same
    /// face id, in case of quads or N-gons.
    ids: Buffer,
    /// A single u32, containing the largest id in the `ids` buffer. Used to
    /// generate the debug view.
    max_id: Buffer,
    /// The number of faces
    len: usize,
}

impl RoutineLayout<ID_NUM_BUFFERS, 0, ID_NUM_UNIFORMS> for FaceIdLayout {
    type Settings = ();

    fn get_wgpu_buffers(&self, _settings: &Self::Settings) -> [&Buffer; ID_NUM_BUFFERS] {
        [&self.positions, &self.ids]
    }

    fn get_wgpu_textures<'a>(
        &'a self,
        _texture_manager: &'a TextureManager,
        _settings: &'a Self::Settings,
    ) -> [&'a TextureView; 0] {
        []
    }

    fn get_wgpu_uniforms(&self, _settings: &Self::Settings) -> [&Buffer; ID_NUM_UNIFORMS] {
        [&self.max_id]
    }

    fn get_draw_type(&self, _settings: &Self::Settings) -> DrawType<'_> {
        DrawType::UseInstances {
            num_vertices: 3,
            num_instances: self.len,
        }
    }
}

pub struct FaceRoutine {
    matcaps: Arc<Vec<String>>,
    base_mesh_routine:
        RoutineRenderer<MeshFacesLayout, BASE_MESH_NUM_BUFFERS, BASE_MESH_NUM_TEXTURES>,
    face_overlay_routine:
        RoutineRenderer<FaceOverlayLayout, OVERLAY_NUM_BUFFERS, 0, OVERLAY_NUM_UNIFORMS>,
    face_id_routine: RoutineRenderer<FaceIdLayout, ID_NUM_BUFFERS, 0, ID_NUM_UNIFORMS>,
}

impl FaceRoutine {
    pub fn new(
        device: &Device,
        texture_manager: &mut TextureManager,
        shader_manager: &ShaderManager,
        multisample_config: MultisampleConfig,
    ) -> Self {
        let mut matcaps = Vec::new();
        macro_rules! load_matcap {
            ($image_name:expr) => {
                let image = image::load_from_memory(include_bytes!(concat!(
                    "../../resources/matcap/",
                    $image_name,
                    ".png"
                )))
                .expect(concat!("loading texture ", $image_name));

                texture_manager.add_texture2d($image_name.to_string(), image);
                matcaps.push($image_name.to_string());
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
            base_mesh_routine: RoutineRenderer::new(
                "base mesh",
                device,
                shader_manager.get("face_draw"),
                PrimitiveTopology::TriangleList,
                FrontFace::Cw,
                multisample_config,
            ),
            face_overlay_routine: RoutineRenderer::new(
                "face overlay",
                device,
                shader_manager.get("face_overlay_draw"),
                PrimitiveTopology::TriangleList,
                FrontFace::Cw,
                multisample_config,
            ),
            face_id_routine: RoutineRenderer::new(
                "face id",
                device,
                shader_manager.get("face_id_draw"),
                PrimitiveTopology::TriangleList,
                FrontFace::Cw,
                // The id map is always drawn without multisampling.
                // We don't care about aliasing there.
                MultisampleConfig::One,
            ),
        }
    }

    pub fn add_base_mesh(
        &mut self,
        device: &Device,
        positions: &[Vec3],
        normals: &[Vec3],
        indices: &[u32],
    ) {
        let num_indices = indices.len();

        assert_eq!(positions.len(), normals.len());

        let positions = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(positions),
            usage: BufferUsages::STORAGE,
        });
        let normals = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(normals),
            usage: BufferUsages::STORAGE,
        });
        let indices = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.base_mesh_routine.layouts.push(MeshFacesLayout {
            positions,
            normals,
            indices,
            matcaps: self.matcaps.clone(),
            num_indices,
        });
    }

    pub fn add_overlay_mesh(
        &mut self,
        device: &Device,
        positions: &[Vec3],
        colors: &[Vec4],
        ids: &[u32],
        max_id: u32,
    ) {
        let len = colors.len();

        assert_eq!(positions.len(), len * 3);
        assert_eq!(colors.len(), len);

        let positions_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(positions),
            usage: BufferUsages::STORAGE,
        });
        // @Perf -- We need to duplicate this to render both the overlay and id
        // routines. This is because the system isn't flexible enough to reuse
        // data from different layouts.
        let positions_buf_cpy = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(positions),
            usage: BufferUsages::STORAGE,
        });
        let colors = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(colors),
            usage: BufferUsages::STORAGE,
        });
        let ids = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(ids),
            usage: BufferUsages::STORAGE,
        });
        let max_id = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&max_id),
            usage: BufferUsages::UNIFORM,
        });

        self.face_overlay_routine.layouts.push(FaceOverlayLayout {
            positions: positions_buf,
            colors,
            len,
        });

        self.face_id_routine.layouts.push(FaceIdLayout {
            positions: positions_buf_cpy,
            ids,
            max_id,
            len,
        });
    }

    pub fn clear(&mut self) {
        self.base_mesh_routine.clear();
        self.face_overlay_routine.clear();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        device: &Device,
        encoder: &mut CommandEncoder,
        texture_manager: &TextureManager,
        render_state: &ViewportRenderState,
        settings: &Viewport3dSettings,
        base_clear_buffer: bool,
        overlay_clear_buffer: bool,
    ) {
        self.base_mesh_routine.render(
            device,
            encoder,
            texture_manager,
            render_state,
            settings,
            &[],
            base_clear_buffer,
            None,
        );
        self.face_overlay_routine.render(
            device,
            encoder,
            texture_manager,
            render_state,
            &(),
            &[],
            overlay_clear_buffer,
            None,
        );
        self.face_id_routine.render(
            device,
            encoder,
            texture_manager,
            render_state,
            &(),
            &[&render_state.id_map_target],
            overlay_clear_buffer,
            Some(&render_state.id_map_depth_target),
        );
    }
}
