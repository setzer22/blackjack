use crate::prelude::*;
use egui::{FontDefinitions, Style};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

use crate::rendergraph::egui_routine_custom::EguiCustomRoutine;

pub struct RootViewport {
    platform: Platform,
    screen_descriptor: ScreenDescriptor,
    renderpass: RenderPass,
    screen_format: r3::TextureFormat,
}

impl RootViewport {
    pub fn new(
        renderer: &r3::Renderer,
        window_size: UVec2,
        scale_factor: f64,
        screen_format: r3::TextureFormat,
    ) -> Self {
        RootViewport {
            platform: Platform::new(PlatformDescriptor {
                physical_width: window_size.x,
                physical_height: window_size.y,
                scale_factor: scale_factor,
                font_definitions: FontDefinitions::default(),
                style: Style::default(),
            }),
            screen_descriptor: ScreenDescriptor {
                physical_width: window_size.x,
                physical_height: window_size.y,
                scale_factor: scale_factor as f32,
            },
            renderpass: RenderPass::new(&renderer.device, screen_format, 0),
            screen_format,
        }
    }

    pub fn on_winit_event(&mut self, event: &winit::event::Event<()>) {
        self.platform.handle_event(event);
        match event {
            winit::event::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::Resized(new_size) => {
                    self.screen_descriptor.physical_width = new_size.width;
                    self.screen_descriptor.physical_height = new_size.height;
                }
                winit::event::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    self.screen_descriptor.scale_factor = *scale_factor as f32;
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn add_draw_to_graph<'node>(
        &'node mut self,
        graph: &mut r3::RenderGraph<'node>,
        output: r3::RenderTargetHandle,
    ) {
        let (_output, paint_commands) = self.platform.end_frame(None);
        let paint_jobs = self.platform.context().tessellate(paint_commands);

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

        let self_pt = builder.passthrough_ref_mut(self);

        builder.build(
            move |pt, renderer, encoder_or_pass, _temps, _ready, graph_data| {
                let this = pt.get_mut(self_pt);
                let rpass = encoder_or_pass.get_rpass(rpass_handle);

                this.renderpass.update_texture(
                    &renderer.device,
                    &renderer.queue,
                    &this.platform.context.font_image(),
                );
                this.renderpass.update_buffers(
                    &renderer.device,
                    &renderer.queue,
                    &paint_jobs,
                    &this.screen_descriptor,
                );

                this.renderpass
                    .execute_with_renderpass(rpass, input.clipped_meshes, &this.screen_descriptor, zoom_level)
                    .unwrap();

            },
        );
    }
}
