use super::*;

impl RootViewport {
    fn add_draw_to_graph<'node>(
        &'node mut self,
        graph: &mut r3::RenderGraph<'node>,
        ready: &r3::ReadyData,
        viewport_routines: ViewportRoutines<'node>,
        output: r3::RenderTargetHandle,
    ) {
        // Self contains too many things to passthrough it to the inner node `.build`
        //  closure, so we split it up here to make borrow checking more granular
        let Self {
            ref mut renderpass,
            ref mut screen_descriptor,
            ref mut platform,
            ref mut graph_editor,
            ref mut offscreen_viewports,
            ref mut viewport_3d,
            ..
        } = self;

        // --- Draw child UIs ---
        let parent_scale = platform.context().pixels_per_point();
        let graph_texture = graph_editor.add_draw_to_graph(
            graph,
            offscreen_viewports[&OffscreenViewport::GraphEditor].rect,
            parent_scale,
        );
        let viewport_3d_texture = viewport_3d.add_to_graph(graph, ready, viewport_routines);

        // --- Draw parent UI ---
        let (_output, paint_commands) = platform.end_frame(None);
        let paint_jobs = platform.context().tessellate(paint_commands);

        let mut builder = graph.add_node("RootViewport");

        let output_handle = builder.add_render_target_output(output);
        let rpass_handle = builder.add_renderpass(r3::RenderPassTargets {
            targets: vec![r3::RenderPassTarget {
                color: output_handle,
                clear: wgpu::Color::BLACK,
                resolve: None,
            }],
            depth_stencil: None,
        });

        let graph_handle = builder.add_render_target_input(graph_texture);
        let viewport_3d_handle = builder.add_render_target_input(viewport_3d_texture);

        let renderpass_pt = builder.passthrough_ref_mut(renderpass);
        let screen_descriptor_pt = builder.passthrough_ref_mut(screen_descriptor);
        let platform_pt = builder.passthrough_ref_mut(platform);
        let offscreen_pt = builder.passthrough_ref_mut(offscreen_viewports);

        builder.build(
            move |pt, renderer, encoder_or_pass, _temps, _ready, graph_data| {
                let renderpass = pt.get_mut(renderpass_pt);
                let screen_descriptor = pt.get_mut(screen_descriptor_pt);
                let platform = pt.get_mut(platform_pt);
                let offscreen_viewports = pt.get_mut(offscreen_pt);

                let rpass = encoder_or_pass.get_rpass(rpass_handle);

                renderpass.update_texture(
                    &renderer.device,
                    &renderer.queue,
                    &platform.context().font_image(),
                );
                renderpass.update_user_textures(&renderer.device, &renderer.queue);
                renderpass.update_buffers(
                    &renderer.device,
                    &renderer.queue,
                    &paint_jobs,
                    &screen_descriptor,
                );

                // --- Register offscreen viewports ---

                // Graph editor
                let graph_texture = graph_data.get_render_target(graph_handle);
                let graph_texture_egui = renderpass.egui_texture_from_wgpu_texture(
                    &renderer.device,
                    graph_texture,
                    wgpu::FilterMode::Linear,
                );
                offscreen_viewports
                    .entry(OffscreenViewport::GraphEditor)
                    .and_modify(|vwp| {
                        vwp.texture_id = Some(graph_texture_egui);
                    });

                // Viewport 3d
                let viewport_3d_texture = graph_data.get_render_target(viewport_3d_handle);
                let viewport_3d_texture_egui = renderpass.egui_texture_from_wgpu_texture(
                    &renderer.device,
                    viewport_3d_texture,
                    wgpu::FilterMode::Linear,
                );
                offscreen_viewports
                    .entry(OffscreenViewport::Viewport3d)
                    .and_modify(|vwp| {
                        vwp.texture_id = Some(viewport_3d_texture_egui);
                    });

                renderpass
                    .execute_with_renderpass(rpass, &paint_jobs, &screen_descriptor, 1.0)
                    .unwrap();
            },
        );
    }

    pub fn add_root_to_graph<'node>(
        &'node mut self,
        graph: &mut r3::RenderGraph<'node>,
        ready: &r3::ReadyData,
        viewport_routines: ViewportRoutines<'node>,
    ) {
        let output = graph.add_surface_texture();
        self.add_draw_to_graph(graph, ready, viewport_routines, output);
    }
}
