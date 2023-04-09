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

use super::{
    id_picking_routine::PickableId,
    render_state::ViewportRenderState,
    routine_renderer::{
        DrawType, MultisampleConfig, RenderCommand, RoutineLayout, RoutineRenderer,
    },
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
    pub object_pick_id: PickableId, // same layout as u32
    pub is_highlighted: u32,        // bool, but we can't have padding
}

/// Represents the required buffers to draw a gizmo mesh.
pub struct GizmosLayout {
    indices: Buffer,
    positions: Buffer,
    subgizmo_ids: Buffer,
    subgizmos: Buffer,
    num_indices: usize,
}

impl RoutineLayout for GizmosLayout {
    type Settings = ();

    fn get_wgpu_buffers(&self, _settings: &()) -> Vec<&Buffer> {
        vec![&self.positions, &self.subgizmo_ids, &self.subgizmos]
    }

    fn get_wgpu_textures<'a>(
        &self,
        _texture_manager: &'a TextureManager,
        _settings: &(),
    ) -> Vec<&'a TextureView> {
        vec![]
    }

    fn get_wgpu_uniforms(&self, _settings: &Self::Settings) -> Vec<&Buffer> {
        vec![]
    }

    fn get_draw_type(&self, _settings: &Self::Settings) -> DrawType<'_> {
        DrawType::UseIndices {
            indices: &self.indices,
            num_indices: self.num_indices,
        }
    }

    fn num_buffers() -> usize {
        3
    }
}

pub struct GizmoRoutine {
    gizmo_color_routine: RoutineRenderer<GizmosLayout>,
    gizmo_id_routine: RoutineRenderer<GizmosLayout>,
}

/// A helper struct to build a GizmoLayout.
#[derive(Default)]
pub struct GizmoMeshBuilder {
    positions: Vec<Vec3>,
    indices: Vec<u32>,
    subgizmo_ids: Vec<u32>,
    next_subgizmo_id: u32,
    subgizmos: Vec<Subgizmo>,
}

impl GizmoMeshBuilder {
    pub fn add_subgizmo_mesh(&mut self, mesh: &HalfEdgeMesh, color: Vec3) {
        let buffers = mesh
            .generate_triangle_buffers_smooth(true)
            .expect("Subgizmo mesh should not fail");
        let index_offset = self.positions.len() as u32;

        self.positions.extend_from_slice(&buffers.positions);
        self.indices
            .extend(buffers.indices.iter().map(|idx| idx + index_offset));
        self.subgizmo_ids
            .extend(std::iter::repeat(self.next_subgizmo_id).take(buffers.positions.len()));

        self.subgizmos.push(Subgizmo {
            color,
            object_pick_id: PickableId::new_subgizmo(self.next_subgizmo_id),
            is_highlighted: 0, // false
        });

        self.next_subgizmo_id += 1;
    }

    pub fn build(self, device: &Device) -> GizmosLayout {
        let indices = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Gizmo Indices"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: BufferUsages::INDEX,
        });
        let positions = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Gizmo Positions"),
            contents: bytemuck::cast_slice(&self.positions),
            usage: BufferUsages::STORAGE,
        });
        let subgizmo_ids = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Gizmo Subgizmo IDs"),
            contents: bytemuck::cast_slice(&self.subgizmo_ids),
            usage: BufferUsages::STORAGE,
        });
        let subgizmos = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Gizmo Subgizmos"),
            contents: bytemuck::cast_slice(&self.subgizmos),
            usage: BufferUsages::STORAGE,
        });

        GizmosLayout {
            indices,
            positions,
            subgizmo_ids,
            subgizmos,
            num_indices: self.indices.len(),
        }
    }
}

impl GizmoRoutine {
    pub fn new(
        device: &Device,
        shader_manager: &ShaderManager,
        multisample_config: MultisampleConfig,
    ) -> Self {
        let color_shader = shader_manager.get("gizmo_color");
        let mut gizmo_color_routine = RoutineRenderer::new(
            "gizmo color",
            device,
            color_shader,
            PrimitiveTopology::TriangleList,
            FrontFace::Cw,
            multisample_config,
        );
        let id_shader = shader_manager.get("gizmo_id");
        let mut gizmo_id_routine = RoutineRenderer::new(
            "gizmo id",
            device,
            id_shader,
            PrimitiveTopology::TriangleList,
            FrontFace::Cw,
            // The id map is always drawn without multisampling.
            // We don't care about aliasing there.
            MultisampleConfig::One,
        );

        // TODO: Placeholder code
        gizmo_color_routine
            .layouts
            .push(Self::build_transform_gizmo(device));
        gizmo_id_routine
            .layouts
            .push(Self::build_transform_gizmo(device));

        GizmoRoutine {
            gizmo_color_routine,
            gizmo_id_routine,
        }
    }

    pub fn build_transform_gizmo(device: &Device) -> GizmosLayout {
        // The arrow mesh, which is used for all three axes. Can be used to
        // translate in a single direction.
        let arrow_mesh = HalfEdgeMesh::from_wavefront_obj_str(include_str!(
            "../../resources/meshes/gizmo_translate_arrow.obj"
        ))
        .expect("Could not open arrow mesh gizmo OBJ");

        // The plane mesh, shown at intersections between axes. Can be used to
        // translate in two simultaneous directions.
        let plane_mesh = HalfEdgeMesh::from_wavefront_obj_str(include_str!(
            "../../resources/meshes/gizmo_translate_plane_handle.obj"
        ))
        .expect("Could not open plane handle mesh gizmo OBJ");

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

        let xy_plane = plane_mesh.clone();
        let xz_plane = plane_mesh.clone();
        let yz_plane = plane_mesh.clone();
        edit_ops::transform(
            &xz_plane,
            Vec3::ZERO,
            Vec3::X * 90.0f32.to_radians(),
            Vec3::ONE,
        )
        .expect("Transform");
        edit_ops::transform(
            &yz_plane,
            Vec3::ZERO,
            Vec3::Y * -90.0f32.to_radians(),
            Vec3::ONE,
        )
        .expect("Transform");

        let mut builder = GizmoMeshBuilder::default();
        builder.add_subgizmo_mesh(&x_axis, Vec3::new(1.0, 0.0, 0.0));
        builder.add_subgizmo_mesh(&y_axis, Vec3::new(0.0, 1.0, 0.0));
        builder.add_subgizmo_mesh(&z_axis, Vec3::new(0.0, 0.0, 1.0));
        builder.add_subgizmo_mesh(&xy_plane, Vec3::new(1.0, 1.0, 0.0));
        builder.add_subgizmo_mesh(&xz_plane, Vec3::new(1.0, 0.0, 1.0));
        builder.add_subgizmo_mesh(&yz_plane, Vec3::new(0.0, 1.0, 1.0));

        builder.build(device)
    }

    pub fn render(
        &self,
        device: &Device,
        encoder: &mut CommandEncoder,
        texture_manager: &TextureManager,
        render_state: &ViewportRenderState,
        clear_buffer: bool,
    ) {
        self.gizmo_color_routine.render(
            device,
            encoder,
            RenderCommand::new(texture_manager, render_state, &()).clear_buffer(clear_buffer),
        );
        self.gizmo_id_routine.render(
            device,
            encoder,
            RenderCommand::new(texture_manager, render_state, &())
                .clear_buffer(clear_buffer)
                .offscren_targets(&[&render_state.id_map_target])
                .override_depth(Some(&render_state.id_map_depth_target)),
        );
    }
}
