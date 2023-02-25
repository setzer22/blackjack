// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use glam::{IVec2, UVec2};
use wgpu::*;

use crate::viewport_3d::Viewport3dSettings;

use self::{
    face_routine::FaceRoutine, id_picking_routine::IdPickingRoutine,
    point_cloud_routine::PointCloudRoutine, render_state::ViewportRenderState,
    texture_manager::TextureManager, wireframe_routine::WireframeRoutine,
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

/// The state for the renderer (bind groups, texture views...) that is common in
/// all routines.
pub mod render_state;

pub struct BlackjackViewportRenderer {
    device: Arc<Device>,
    wireframe_routine: WireframeRoutine,
    point_cloud_routine: PointCloudRoutine,
    face_routine: FaceRoutine,
    id_picking_routine: IdPickingRoutine,
}

pub struct ViewportRendererOutput {
    pub color_texture: Texture,
    pub id_under_mouse: Option<u32>,
}

impl BlackjackViewportRenderer {
    pub fn render(
        &self,
        resolution: UVec2,
        texture_manager: &TextureManager,
        settings: &Viewport3dSettings,
    ) -> ViewportRendererOutput {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("blackjack viewport renderer"),
            });

        let (color, color_view) = wgpu_utils::create_render_texture(
            &self.device,
            "Blackjack Viewport Color",
            resolution,
            None,
        );
        let (_depth, depth_view) = wgpu_utils::create_render_texture(
            &self.device,
            "Blackjack Viewport Depth",
            resolution,
            Some(TextureFormat::Depth32Float),
        );
        let (id_map, id_map_view) = wgpu_utils::create_render_texture(
            &self.device,
            "id_map texture",
            resolution,
            Some(
                TextureFormat::R32Uint, // Should match one in shader manager
            ),
        );

        let render_state =
            ViewportRenderState::new(&self.device, resolution, color_view, depth_view);

        self.wireframe_routine
            .render(&self.device, &mut encoder, texture_manager, &render_state);
        self.point_cloud_routine
            .render(&self.device, &mut encoder, texture_manager, &render_state);
        self.face_routine.render(
            &self.device,
            &mut encoder,
            texture_manager,
            &render_state,
            settings,
            &id_map_view,
        );
        self.id_picking_routine
            .run(&mut encoder, resolution, &id_map);

        ViewportRendererOutput {
            color_texture: color,
            id_under_mouse: self.id_picking_routine.id_under_mouse(&self.device),
        }
    }
}
