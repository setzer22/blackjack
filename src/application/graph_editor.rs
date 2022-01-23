use crate::{graph::graph_editor_egui::viewport_manager::AppViewport, prelude::*};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

use crate::graph::graph_editor_egui::editor_state::EditorState;

pub struct GraphEditor {
    pub state: EditorState,
    pub platform: Platform,
    pub renderpass: RenderPass,
    pub zoom_level: f32,
}

impl GraphEditor {
    pub const ZOOM_LEVEL_MIN: f32 = 0.5;
    pub const ZOOM_LEVEL_MAX: f32 = 10.0;

    pub fn new(device: &wgpu::Device, window_size: UVec2, format: r3::TextureFormat) -> Self {
        Self {
            state: EditorState::new(),
            platform: Platform::new(PlatformDescriptor {
                // The width here is not really relevant, and will be reset on
                // the next resize event.
                physical_width: window_size.x,
                physical_height: window_size.y,
                // There is no scaling on child egui instances
                scale_factor: 1.0,
                font_definitions: egui::FontDefinitions::default(),
                style: egui::Style::default(),
            }),
            renderpass: RenderPass::new(device, format, 1),
            zoom_level: 1.0,
        }
    }

    /// Handles most window events, but ignores resize / dpi change events,
    /// because this is not a root-level egui instance.
    ///
    /// Mouse events are translated according to the inner `viewport`
    pub fn on_winit_event(
        &mut self,
        parent_scale: f32,
        viewport_rect: egui::Rect,
        event: winit::event::Event<'static, ()>,
    ) {
        // Copy event so we can modify it locally
        let mut event = event.clone();

        match event {
            winit::event::Event::WindowEvent { ref mut event, .. } => match event {
                // Filter out scaling / resize events
                winit::event::WindowEvent::Resized(_)
                | winit::event::WindowEvent::ScaleFactorChanged { .. } => return,
                // Hijack mouse events so they are relative to the viewport and
                // account for zoom level.
                winit::event::WindowEvent::CursorMoved {
                    ref mut position, ..
                } => {
                    position.x -= (viewport_rect.min.x * parent_scale) as f64;
                    position.y -= (viewport_rect.min.y * parent_scale) as f64;
                    position.x *= self.zoom_level as f64;
                    position.y *= self.zoom_level as f64;
                }

                winit::event::WindowEvent::MouseWheel { delta, .. } => match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, dy) => {
                        self.zoom_level += *dy as f32 * 8.0 * 0.01;
                        self.zoom_level = self
                            .zoom_level
                            .clamp(Self::ZOOM_LEVEL_MIN, Self::ZOOM_LEVEL_MAX);
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        self.zoom_level -= pos.y as f32 * 0.01;
                        self.zoom_level = self
                            .zoom_level
                            .clamp(Self::ZOOM_LEVEL_MIN, Self::ZOOM_LEVEL_MAX);
                    }
                },
                _ => {}
            },
            _ => {}
        }

        self.platform.handle_event(&event);
    }

    pub fn resize_platform(&mut self, parent_scale: f32, viewport_rect: egui::Rect) {
        // We craft a fake resize event so that the code in egui_winit_platform
        // remains unchanged, thinking it lives in a real window. The poor thing!
        let fake_resize_event: winit::event::Event<()> = winit::event::Event::WindowEvent {
            // SAFETY: This dummy window id is never getting sent back to winit
            window_id: unsafe { winit::window::WindowId::dummy() },
            event: winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize::new(
                (viewport_rect.width() * self.zoom_level * parent_scale) as u32,
                (viewport_rect.height() * self.zoom_level * parent_scale) as u32,
            )),
        };
        self.platform.handle_event(&fake_resize_event);
    }

    pub fn update(&mut self, parent_scale: f32, viewport_rect: egui::Rect) {
        self.resize_platform(parent_scale, viewport_rect);
        self.platform.raw_input.pixels_per_point = Some(1.0 / dbg!(self.zoom_level));
        self.platform.begin_frame();

        let ctx = self.platform.context();

        crate::graph::graph_editor_egui::draw_graph_editor(&ctx, &mut self.state);

        // Debug mouse pointer position
        // -- This is useful when mouse events are not being interpreted correctly.
        if let Some(pos) = ctx.input().pointer.hover_pos() {
            ctx.debug_painter()
                .circle(pos, 5.0, egui::Color32::GREEN, egui::Stroke::none());
        }
    }

    pub fn screen_descriptor(
        &self,
        viewport_rect: egui::Rect,
        parent_scale: f32,
    ) -> ScreenDescriptor {
        ScreenDescriptor {
            physical_width: (viewport_rect.width() * parent_scale * self.zoom_level) as u32,
            physical_height: (viewport_rect.height() * parent_scale * self.zoom_level) as u32,
            scale_factor: 1.0,
        }
    }

    pub fn add_draw_to_graph<'node>(
        &'node mut self,
        graph: &mut r3::RenderGraph<'node>,
        viewport_rect: egui::Rect,
        parent_scale: f32,
    ) -> r3::RenderTargetHandle {
        let resolution = viewport_rect.size() * parent_scale;
        let resolution = UVec2::new(resolution.x as u32, resolution.y as u32);

        let render_target = graph.add_render_target(r3::RenderTargetDescriptor {
            label: None,
            resolution,
            samples: r3::SampleCount::One,
            format: r3::TextureFormat::Bgra8UnormSrgb,
            usage: r3::TextureUsages::RENDER_ATTACHMENT | r3::TextureUsages::TEXTURE_BINDING,
        });

        let (_output, paint_commands) = self.platform.end_frame(None);
        let paint_jobs = self.platform.context().tessellate(paint_commands);

        let mut builder = graph.add_node("RootViewport");

        let output_handle = builder.add_render_target_output(render_target);
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
            move |pt, renderer, encoder_or_pass, _temps, _ready, _graph_data| {
                let this = pt.get_mut(self_pt);
                let rpass = encoder_or_pass.get_rpass(rpass_handle);

                let screen_descriptor = this.screen_descriptor(viewport_rect, parent_scale);

                this.renderpass.update_texture(
                    &renderer.device,
                    &renderer.queue,
                    &this.platform.context().font_image(),
                );
                this.renderpass
                    .update_user_textures(&renderer.device, &renderer.queue);
                this.renderpass.update_buffers(
                    &renderer.device,
                    &renderer.queue,
                    &paint_jobs,
                    &screen_descriptor,
                );

                this.renderpass
                    .execute_with_renderpass(
                        rpass,
                        &paint_jobs,
                        &screen_descriptor,
                        this.zoom_level,
                    )
                    .unwrap();
            },
        );

        render_target
    }
}
