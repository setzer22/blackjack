use std::{rc::Rc, sync::Arc};

use crate::{app_window, graph::graph_editor_egui::viewport_manager::AppViewport, prelude::*};
use egui::{FontDefinitions, Style};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use rend3::RenderTargetHandle;

use self::{application_context::ApplicationContext, graph_editor::GraphEditor};

pub struct RootViewport {
    platform: Platform,
    screen_descriptor: ScreenDescriptor,
    renderpass: RenderPass,
    screen_format: r3::TextureFormat,
    app_context: application_context::ApplicationContext,
    graph_editor: graph_editor::GraphEditor,
    /// Stores the egui texture ids for the child viewports.
    offscreen_viewports: HashMap<OffscreenViewport, AppViewport>,
}

/// The application context is state that is global to an instance of blackjack.
/// The currently open file and any data that is not per-viewport goes here.
pub mod application_context;

/// The graph editor viewport. Shows an inner egui instance with zooming /
/// panning functionality.
pub mod graph_editor;

/// The 3d viewport, shows the current mesh
pub mod viewport_3d;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum OffscreenViewport {
    GraphEditor,
    Viewport3d,
}

impl RootViewport {
    pub fn new(
        renderer: &r3::Renderer,
        window_size: UVec2,
        scale_factor: f64,
        screen_format: r3::TextureFormat,
    ) -> Self {
        // NOTE: As it is now, offscreen_viewports could simply be a struct. The
        // reason it's a HashMap is because in the future there will be multiple
        // GraphEditors and Viewport3ds, and the hashmap key will be the
        // viewport id instead.
        let mut offscreen_viewports = HashMap::new();
        offscreen_viewports.insert(OffscreenViewport::GraphEditor, AppViewport::new());
        offscreen_viewports.insert(OffscreenViewport::Viewport3d, AppViewport::new());

        RootViewport {
            platform: Platform::new(PlatformDescriptor {
                physical_width: window_size.x,
                physical_height: window_size.y,
                scale_factor,
                font_definitions: FontDefinitions::default(),
                style: Style::default(),
            }),
            screen_descriptor: ScreenDescriptor {
                physical_width: window_size.x,
                physical_height: window_size.y,
                scale_factor: scale_factor as f32,
            },
            renderpass: RenderPass::new(&renderer.device, screen_format, 1),
            screen_format,
            app_context: ApplicationContext::new(),
            graph_editor: GraphEditor::new(&renderer.device, window_size, screen_format),
            offscreen_viewports,
        }
    }

    pub fn on_winit_event(&mut self, event: winit::event::Event<()>) {
        // NOTE: Winit has a feature we don't use, which causes additional
        // complexity. The ScaleFactorChanged event contains a mutable reference
        // because that's the way to tell winit how we want to resize a window
        // in response to a dpi change event.
        //
        // This means the Event struct is not easily clonable, which prevents
        // some tricks we do to pass and modify events to the inner egui
        // instances. To fix it, there's the `to_static` method which will
        // consume the event and allow cloning for all variants except
        // ScaleFactorChanged.

        match event {
            winit::event::Event::WindowEvent { ref event, .. } => match event {
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

        self.platform.handle_event(&event);
        if let Some(event_static) = event.to_static() {
            // event_static here will never be a ScaleFactorChanged
            let parent_scale = self.screen_descriptor.scale_factor;
            self.graph_editor.on_winit_event(
                parent_scale,
                self.offscreen_viewports[&OffscreenViewport::GraphEditor].rect,
                event_static,
            );
        }
    }

    pub fn update(&mut self) {
        self.graph_editor.update(
            self.screen_descriptor.scale_factor,
            self.offscreen_viewports[&OffscreenViewport::GraphEditor].rect,
        );

        self.platform.begin_frame();
        egui::CentralPanel::default().show(&self.platform.context(), |ui| {
            let mut split_tree = self.app_context.split_tree.clone();
            split_tree.show(ui, self, Self::show_leaf);
            self.app_context.split_tree = split_tree;
        });
    }

    pub fn show_leaf(ui: &mut egui::Ui, payload: &mut Self, name: &str) {
        // TODO: These names here are hard-coded in the creation of the
        // SplitTree. We should be using some kind of identifier instead
        match name {
            "3d_view" => {
                ui.label("3d view");
            }
            "graph_editor" => {
                payload
                    .offscreen_viewports
                    .get_mut(&OffscreenViewport::GraphEditor)
                    .unwrap()
                    .show(ui, ui.available_size());
            }
            "inspector" => {
                ui.label("Properties inspector goes here");
            }
            _ => panic!("Invalid split name {}", name),
        }
    }

    fn add_draw_to_graph<'node>(
        &'node mut self,
        graph: &mut r3::RenderGraph<'node>,
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
            ..
        } = self;

        // --- Draw child UIs ---
        let parent_scale = platform.context().pixels_per_point();
        let graph_texture = graph_editor.add_draw_to_graph(
            graph,
            offscreen_viewports[&OffscreenViewport::GraphEditor].rect,
            parent_scale,
        );

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

                // Register offscreen viewports
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

                renderpass
                    .execute_with_renderpass(rpass, &paint_jobs, &screen_descriptor, 1.0)
                    .unwrap();
            },
        );
    }

    pub fn add_root_to_graph<'node>(&'node mut self, graph: &mut r3::RenderGraph<'node>) {
        //let offscreen = self.add_offscreen_viewports(graph);

        let output = graph.add_surface_texture();
        self.add_draw_to_graph(graph, output);
    }

    pub fn render(&mut self, render_ctx: &mut RenderContext) {
        let frame = rend3::util::output::OutputFrame::Surface {
            surface: Arc::clone(&render_ctx.surface),
        };
        let (cmd_bufs, ready) = render_ctx.renderer.ready();
        let mut graph = rend3::RenderGraph::new();
        self.add_root_to_graph(&mut graph);
        graph.execute(&render_ctx.renderer, frame, cmd_bufs, &ready);
    }
}
