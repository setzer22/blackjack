// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    application::{viewport_3d::Viewport3dSettings, ViewportRoutines},
    prelude::*,
};

pub mod grid_routine;

/// Some common definitions to abstract wgpu boilerplate
pub mod common;

/// The common bits in all the 3d viewport routines
pub mod viewport_3d_routine;

/// A render routine to draw wireframe meshes
pub mod wireframe_routine;

/// A render routine to draw point clouds
pub mod point_cloud_routine;

/// A render routine to draw meshes
pub mod face_routine;

/// A routine to implement object picking, by reading the id_map buffer.
pub mod id_picking_routine;

/// Shader manager struct which sets up loading with a basic preprocessor
pub mod shader_manager;

/// Adds the necessary nodes to render the 3d viewport of the app. The viewport
/// is rendered into a render target, and its handle is returned.
#[allow(clippy::too_many_arguments)]
pub fn blackjack_viewport_rendergraph<'node>(
    graph: &mut r3::RenderGraph<'node>,
    ready: &r3::ReadyData,
    routines: ViewportRoutines<'node>,
    resolution: UVec2,
    samples: r3::SampleCount,
    ambient: Vec4,
    settings: &'node Viewport3dSettings,
) -> r3::RenderTargetHandle {
    // Create intermediate storage
    let state = r3::BaseRenderGraphIntermediateState::new(graph, ready, resolution, samples);

    state.clear(graph, Vec4::new(0.027851, 0.027851, 0.027851, 1.0));

    // Preparing and uploading data
    state.pbr_pre_culling(graph);
    state.create_frame_uniforms(graph, routines.base_graph, ambient, resolution);

    // Culling
    state.pbr_shadow_culling(graph, routines.base_graph, routines.pbr);
    state.pbr_culling(graph, routines.base_graph, routines.pbr);

    // Depth-only rendering
    state.pbr_prepass_rendering(graph, routines.pbr, samples);

    // Forward rendering
    state.pbr_forward_rendering(graph, routines.pbr, samples);

    let id_map = graph.add_render_target(r3::RenderTargetDescriptor {
        label: None,
        resolution,
        samples: r3::SampleCount::One,
        format: r3::TextureFormat::R32Uint, // Should match one in shader manager
        usage: r3::TextureUsages::RENDER_ATTACHMENT
            | r3::TextureUsages::TEXTURE_BINDING
            | r3::TextureUsages::COPY_SRC,
    });

    use crate::application::viewport_3d::EdgeDrawMode::*;
    if matches!(settings.edge_mode, FullEdge | HalfEdge) {
        routines.wireframe.add_to_graph(graph, &state);
    }
    if settings.render_vertices {
        routines.point_cloud.add_to_graph(graph, &state);
    }
    use crate::application::viewport_3d::FaceDrawMode::*;
    if matches!(settings.face_mode, Flat | Smooth | Real) {
        routines.face.add_to_graph(graph, &state, id_map, settings);
    }

    routines.id_picking.add_to_graph(graph, resolution, id_map);

    routines.grid.add_to_graph(graph, &state);

    // Make the reference to the surface
    let output = graph.add_render_target(r3::RenderTargetDescriptor {
        label: Some("Blackjack Viewport Output".into()),
        resolution,
        samples,
        format: r3::TextureFormat::Bgra8UnormSrgb,
        usage: r3::TextureUsages::RENDER_ATTACHMENT | r3::TextureUsages::TEXTURE_BINDING,
    });
    state.tonemapping(graph, routines.tonemapping, output);

    output
}
