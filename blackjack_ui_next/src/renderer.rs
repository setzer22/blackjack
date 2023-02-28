// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use glam::UVec2;
use wgpu::*;

use crate::viewport_3d::Viewport3dSettings;

use self::{
    face_routine::FaceRoutine,
    grid_routine::GridRoutine,
    id_picking_routine::IdPickingRoutine,
    point_cloud_routine::PointCloudRoutine,
    render_state::{ViewportRenderState, ViewportUniforms},
    routine_renderer::MultisampleConfig,
    shader_manager::ShaderManager,
    texture_manager::TextureManager,
    wireframe_routine::WireframeRoutine,
};

/// Some common definitions to abstract wgpu boilerplate
pub mod wgpu_utils;

/// Shader manager struct which sets up loading with a basic preprocessor
pub mod shader_manager;

/// Texture manager struct which can load images in various formats.
pub mod texture_manager;

/// The common bits in all the 3d viewport routines
pub mod routine_renderer;

/// A render routine to draw wireframe meshes
pub mod wireframe_routine;

/// A render routine to draw point clouds
pub mod point_cloud_routine;

/// A render routine to draw meshes
pub mod face_routine;

/// A routine to implement object picking, by reading the id_map buffer.
pub mod id_picking_routine;

/// A routine to render an infinite grid on the XZ plane.
pub mod grid_routine;

/// The state for the renderer (bind groups, texture views...) that is common in
/// all routines.
pub mod render_state;

pub struct BlackjackViewportRenderer {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub shader_manager: ShaderManager,
    pub texture_manager: TextureManager,
    pub wireframe_routine: WireframeRoutine,
    pub point_cloud_routine: PointCloudRoutine,
    pub face_routine: FaceRoutine,
    pub id_picking_routine: IdPickingRoutine,
    pub grid_routine: GridRoutine,
    pub multisample_config: MultisampleConfig,
}

pub struct ViewportRendererOutput {
    pub color_texture_view: TextureView,
    pub id_under_mouse: Option<u32>,
}

pub struct ViewportCamera {
    pub view_matrix: glam::Mat4,
    pub projection_matrix: glam::Mat4,
}

impl BlackjackViewportRenderer {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        multisample_config: MultisampleConfig,
    ) -> Self {
        let shader_manager = ShaderManager::new(&device);
        let mut texture_manager = TextureManager::new(Arc::clone(&device), Arc::clone(&queue));

        Self {
            wireframe_routine: WireframeRoutine::new(&device, &shader_manager, multisample_config),
            point_cloud_routine: PointCloudRoutine::new(
                &device,
                &shader_manager,
                multisample_config,
            ),
            face_routine: FaceRoutine::new(
                &device,
                &mut texture_manager,
                &shader_manager,
                multisample_config,
            ),
            id_picking_routine: IdPickingRoutine::new(&device),
            grid_routine: GridRoutine::new(&device, &shader_manager, multisample_config),
            shader_manager,
            texture_manager,
            device,
            queue,
            multisample_config,
        }
    }

    pub fn render(
        &self,
        resolution: UVec2,
        camera: ViewportCamera,
        settings: &Viewport3dSettings,
    ) -> ViewportRendererOutput {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("blackjack viewport renderer"),
            });

        let color = wgpu_utils::create_multisampled_render_texture(
            &self.device,
            "Blackjack Viewport Multisample Color",
            resolution,
            None,
            self.multisample_config,
        );
        let depth = wgpu_utils::create_multisampled_render_texture(
            &self.device,
            "Blackjack Viewport Depth",
            resolution,
            Some(TextureFormat::Depth32Float),
            self.multisample_config,
        );
        let (id_map, id_map_view) = wgpu_utils::create_render_texture(
            &self.device,
            "id_map texture",
            resolution,
            Some(
                TextureFormat::R32Uint, // Should match one in shader manager
            ),
            1,
        );
        // The id map needs its own depth texture because we can't use the
        // multisampled one.
        let (_id_depth, id_depth_view) = wgpu_utils::create_render_texture(
            &self.device,
            "id_map depth",
            resolution,
            Some(TextureFormat::Depth32Float),
            1,
        );

        let render_state = ViewportRenderState::new(
            &self.device,
            resolution,
            color.view,
            depth.view,
            color.resolve_view,
            ViewportUniforms::new(camera.view_matrix, camera.projection_matrix, resolution),
        );

        self.wireframe_routine.render(
            &self.device,
            &mut encoder,
            &self.texture_manager,
            &render_state,
            true,
        );
        self.point_cloud_routine.render(
            &self.device,
            &mut encoder,
            &self.texture_manager,
            &render_state,
            false,
        );
        self.face_routine.render(
            &self.device,
            &mut encoder,
            &self.texture_manager,
            &render_state,
            settings,
            &id_map_view,
            &id_depth_view,
            false,
            false,
        );
        self.id_picking_routine
            .run(&mut encoder, resolution, &id_map);

        self.grid_routine.render(&mut encoder, &render_state);

        // Send it to the GPU
        self.queue.submit(std::iter::once(encoder.finish()));

        ViewportRendererOutput {
            color_texture_view: match self.multisample_config {
                MultisampleConfig::One => render_state.color_target,
                MultisampleConfig::Four => render_state.color_resolve_target.unwrap(),
            },
            id_under_mouse: self.id_picking_routine.id_under_mouse(&self.device),
        }
    }
}