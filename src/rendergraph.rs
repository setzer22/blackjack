use crate::prelude::*;

pub mod grid_routine;

#[allow(clippy::too_many_arguments)]
pub fn blackjack_rendergraph<'node>(
    base: &'node r3::BaseRenderGraph,
    graph: &mut r3::RenderGraph<'node>,
    ready: &r3::ReadyData,
    pbr: &'node r3::PbrRoutine,
    tonemapping: &'node r3::TonemappingRoutine,
    grid: &'node grid_routine::GridRoutine,
    resolution: UVec2,
    samples: r3::SampleCount,
    ambient: Vec4,
) {
    // Create intermediate storage
    let state = r3::BaseRenderGraphIntermediateState::new(graph, ready, resolution, samples);

    // Preparing and uploading data
    state.pbr_pre_culling(graph);
    state.create_frame_uniforms(graph, base, ambient);

    // Culling
    state.pbr_shadow_culling(graph, base, pbr);
    state.pbr_culling(graph, base, pbr);

    // Depth-only rendering
    state.pbr_prepass_rendering(graph, pbr, samples);

    // Forward rendering
    state.pbr_forward_rendering(graph, pbr, samples);

    grid.add_to_graph(graph, &state);

    // Make the reference to the surface
    let surface = graph.add_surface_texture();
    state.tonemapping(graph, tonemapping, surface);
}
