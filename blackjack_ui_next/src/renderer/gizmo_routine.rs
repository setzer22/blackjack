// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use blackjack_engine::prelude::{edit_ops, HalfEdgeMesh};
use glam::Vec3;

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

/// A subgizmo represents a piece of a gizmo. Vertices of a gizmo are annotated
/// with a subgizmo id, so the shader can reference this data structure. This is
/// used to highlight different parts of the gizmo and to do object picking.
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct Subgizmo {
    pub color: Vec3,
    pub object_pick_id: u32,
    pub is_highlighted: u32, // bool, but we can't have padding
}

/// Represents the required buffers to draw a gizmo mesh.
pub struct GizmosLayout {
    indices: Buffer,
    positions: Buffer,
    subgizmo_ids: Buffer,
    subgizmos: Buffer,
    num_indices: usize,
}

const GIZMO_NUM_BUFFERS: usize = 3;
impl RoutineLayout<GIZMO_NUM_BUFFERS> for GizmosLayout {
    type Settings = Viewport3dSettings;

    fn get_wgpu_buffers(&self, _settings: &Viewport3dSettings) -> [&Buffer; GIZMO_NUM_BUFFERS] {
        [&self.positions, &self.subgizmo_ids, &self.subgizmos]
    }

    fn get_wgpu_textures<'a>(
        &'a self,
        _texture_manager: &'a TextureManager,
        _settings: &Viewport3dSettings,
    ) -> [&'a TextureView; 0] {
        []
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

pub struct GizmoRoutine {
    gizmo_routine: RoutineRenderer<GizmosLayout, GIZMO_NUM_BUFFERS>,
    transform_gizmo: GizmosLayout,
}

impl GizmoRoutine {
    pub fn new(
        device: &Device,
        shader_manager: &ShaderManager,
        multisample_config: MultisampleConfig,
    ) -> Self {
        let shader = shader_manager.get("gizmo_color");

        let mut routine = RoutineRenderer::new(
            "Gizmo Routine",
            device,
            shader,
            PrimitiveTopology::TriangleList,
            FrontFace::Cw,
            multisample_config,
        );

        routine.layouts.push(Self::build_transform_gizmo(device));

        GizmoRoutine {
            gizmo_routine: routine,
            transform_gizmo: Self::build_transform_gizmo(device),
        }
    }

    pub fn build_transform_gizmo(device: &Device) -> GizmosLayout {
        // Build translation gizmo
        let arrow_mesh = HalfEdgeMesh::from_wavefront_obj_str(include_str!(
            "../../resources/meshes/gizmo_translate_arrow.obj"
        ))
        .expect("Could not open arrow mesh gizmo OBJ");

        // Arrow OBJ is stored looking upward (Y axis)
        let x_axis = arrow_mesh.clone();
        let y_axis = arrow_mesh.clone();
        let z_axis = arrow_mesh.clone();
        edit_ops::transform(
            &x_axis,
            Vec3::ZERO,
            Vec3::Z * -90.0f32.to_radians(),
            Vec3::ONE,
        )
        .expect("Transform");
        edit_ops::transform(
            &z_axis,
            Vec3::ZERO,
            Vec3::X * 90.0f32.to_radians(),
            Vec3::ONE,
        )
        .expect("Transform");

        // Convert to buffers
        let mut indices = vec![];
        let mut positions = vec![];
        let mut subgizmo_ids = vec![];

        let x_buffers = x_axis
            .generate_triangle_buffers_smooth(true)
            .expect("Buffers");
        let y_buffers = y_axis
            .generate_triangle_buffers_smooth(true)
            .expect("Buffers");
        let z_buffers = z_axis
            .generate_triangle_buffers_smooth(true)
            .expect("Buffers");

        positions.extend_from_slice(&x_buffers.positions);
        positions.extend_from_slice(&y_buffers.positions);
        positions.extend_from_slice(&z_buffers.positions);

        indices.extend_from_slice(&x_buffers.indices);
        indices.extend(
            y_buffers
                .indices
                .iter()
                .map(|idx| idx + x_buffers.positions.len() as u32),
        );
        indices.extend(
            z_buffers.indices.iter().map(|idx| {
                idx + x_buffers.positions.len() as u32 + y_buffers.positions.len() as u32
            }),
        );

        subgizmo_ids.extend(std::iter::repeat(0u32).take(x_buffers.positions.len()));
        subgizmo_ids.extend(std::iter::repeat(1u32).take(y_buffers.positions.len()));
        subgizmo_ids.extend(std::iter::repeat(2u32).take(z_buffers.positions.len()));

        let subgizmos = vec![
            // X arrow handle
            Subgizmo {
                color: Vec3::new(1.0, 0.0, 0.0),
                object_pick_id: 0,
                is_highlighted: 0,
            },
            // Y arrow handle
            Subgizmo {
                color: Vec3::new(0.0, 1.0, 0.0),
                object_pick_id: 1,
                is_highlighted: 0,
            },
            // Z arrow handle
            Subgizmo {
                color: Vec3::new(0.0, 0.0, 1.0),
                object_pick_id: 2,
                is_highlighted: 0,
            },
        ];

        GizmosLayout {
            num_indices: indices.len(),
            indices: device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&indices),
                usage: BufferUsages::INDEX,
            }),
            positions: device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&positions),
                usage: BufferUsages::STORAGE,
            }),
            subgizmo_ids: device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&subgizmo_ids),
                usage: BufferUsages::STORAGE,
            }),
            subgizmos: device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&subgizmos),
                usage: BufferUsages::STORAGE,
            }),
        }
    }

    pub fn render(
        &self,
        device: &Device,
        encoder: &mut CommandEncoder,
        texture_manager: &TextureManager,
        render_state: &ViewportRenderState,
        settings: &Viewport3dSettings,
        clear_buffer: bool,
    ) {
        self.gizmo_routine.render(
            device,
            encoder,
            texture_manager,
            render_state,
            settings,
            &[],
            clear_buffer,
            None,
        );
    }
}
