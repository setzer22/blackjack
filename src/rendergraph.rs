use crate::{prelude::*, render_context::EguiTextures};

pub mod grid_routine;
pub mod egui_routine_custom;

/// Adds the necessary nodes to render the 3d viewport of the app. The viewport
/// is rendered into a render target, and its handle is returned.
#[allow(clippy::too_many_arguments)]
pub fn blackjack_viewport_rendergraph<'node>(
    base: &'node r3::BaseRenderGraph,
    graph: &mut r3::RenderGraph<'node>,
    ready: &r3::ReadyData,
    pbr: &'node r3::PbrRoutine,
    tonemapping: &'node r3::TonemappingRoutine,
    grid: &'node grid_routine::GridRoutine,
    resolution: UVec2,
    samples: r3::SampleCount,
    ambient: Vec4,
) -> r3::RenderTargetHandle {
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
    let output = graph.add_render_target(r3::RenderTargetDescriptor {
        label: Some("Blackjack Viewport Output".into()),
        resolution,
        samples,
        format: r3::TextureFormat::Bgra8UnormSrgb,
        usage: r3::TextureUsages::RENDER_ATTACHMENT | r3::TextureUsages::TEXTURE_BINDING,
    });
    state.tonemapping(graph, tonemapping, output);

    output
}

/// Some parts of the interface, like the 3d viewport, are rendered as offscreen
/// textures. This function adds the necessary nodes to upload those textures
/// for the final egui pass, where they will be rendered as `egui::Image`s
pub fn upload_offscreen_textures_to_egui<'node>(
    graph: &mut r3::RenderGraph<'node>,
    egui: &'node mut r3::EguiRenderRoutine,
    egui_textures: &'node mut EguiTextures,
    viewport: r3::RenderTargetHandle,
) {
    let mut builder = graph.add_node("Upload offscreen textures to egui");

    let viewport = builder.add_render_target_input(viewport);

    let egui_pt = builder.passthrough_ref_mut(egui);
    let textures_pt = builder.passthrough_ref_mut(egui_textures);

    builder.build(
        move |pt, renderer, _encoder_or_pass, _temps, _ready, graph_data| {
            let egui = pt.get_mut(egui_pt);
            let textures = pt.get_mut(textures_pt);

            let viewport_texture = graph_data.get_render_target(viewport);

            textures.viewport = Some(egui.internal.egui_texture_from_wgpu_texture(
                &renderer.device,
                viewport_texture,
                wgpu::FilterMode::Linear,
            ));
        },
    );
}
