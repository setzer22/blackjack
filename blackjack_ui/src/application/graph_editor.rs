// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{app_window::input::viewport_relative_position, prelude::*};
use blackjack_engine::graph::NodeDefinitions;
use egui_wgpu::renderer::{RenderPass, ScreenDescriptor};

use super::blackjack_theme;

pub struct GraphEditor {
    pub editor_state: graph::GraphEditorState,
    pub custom_state: graph::CustomGraphState,
    pub egui_context: egui::Context,
    pub egui_winit_state: egui_winit::State,
    pub renderpass: RenderPass,
    pub raw_mouse_position: Option<egui::Pos2>,
    pub textures_to_free: Vec<egui::TextureId>,
}

pub fn blackjack_graph_theme() -> egui::Visuals {
    let mut visuals = blackjack_theme();
    visuals.widgets.noninteractive.bg_fill = color_from_hex("#212121").unwrap();
    visuals
}

impl GraphEditor {
    pub const ZOOM_LEVEL_MIN: f32 = 0.5;
    pub const ZOOM_LEVEL_MAX: f32 = 10.0;

    pub fn new(
        renderer: &r3::Renderer,
        format: r3::TextureFormat,
        parent_scale: f32,
        node_definitions: NodeDefinitions,
    ) -> Self {
        let egui_context = egui::Context::default();
        egui_context.set_visuals(blackjack_graph_theme());

        let mut egui_winit_state = egui_winit::State::new_with_wayland_display(None);
        egui_winit_state.set_max_texture_side(renderer.limits.max_texture_dimension_2d as usize);
        egui_winit_state.set_pixels_per_point(1.0);

        Self {
            // Set default zoom to the inverse of ui scale to preserve dpi
            editor_state: graph::GraphEditorState::new(1.0 / parent_scale),
            custom_state: graph::CustomGraphState::new(node_definitions),
            egui_context,
            egui_winit_state,
            renderpass: RenderPass::new(&renderer.device, format, 1),
            // The mouse position, in window coordinates. Stored to hide other
            // window events from egui when the cursor is not over the viewport
            raw_mouse_position: None,
            textures_to_free: Vec::new(),
        }
    }

    pub fn zoom_level(&self) -> f32 {
        self.editor_state.pan_zoom.zoom
    }

    /// Handles most window events, but ignores resize / dpi change events,
    /// because this is not a root-level egui instance.
    ///
    /// Mouse events are translated according to the inner `viewport`
    pub fn on_winit_event(
        &mut self,
        parent_scale: f32,
        viewport_rect: egui::Rect,
        mut event: winit::event::WindowEvent,
        mouse_captured_elsewhere: bool,
    ) {
        let mouse_in_viewport = !mouse_captured_elsewhere
            && self
                .raw_mouse_position
                .map(|pos| viewport_rect.scale_from_origin(parent_scale).contains(pos))
                .unwrap_or(false);

        match &mut event {
            // Filter out scaling / resize events
            winit::event::WindowEvent::Resized(_)
            | winit::event::WindowEvent::ScaleFactorChanged { .. } => return,
            // Hijack mouse events so they are relative to the viewport and
            // account for zoom level.
            winit::event::WindowEvent::CursorMoved {
                ref mut position, ..
            } => {
                self.raw_mouse_position =
                    Some(egui::Pos2::new(position.x as f32, position.y as f32));
                *position = viewport_relative_position(
                    *position,
                    parent_scale,
                    viewport_rect,
                    self.zoom_level(),
                );
            }
            // Ignore mouse press events when clicking outside the editor
            // area. This prevents a bug where clicking on the inspector
            // window while a node is selected disables the current
            // selection.
            winit::event::WindowEvent::MouseInput {
                state: winit::event::ElementState::Pressed,
                ..
            } if !mouse_in_viewport => return,

            winit::event::WindowEvent::MouseWheel { delta, .. } if mouse_in_viewport => {
                let mouse_pos = if let Some(raw_pos) = self.raw_mouse_position {
                    viewport_relative_position(raw_pos.to_winit(), parent_scale, viewport_rect, 1.0)
                        .to_egui()
                } else {
                    egui::pos2(0.0, 0.0)
                }
                .to_vec2();
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, dy) => {
                        self.editor_state.pan_zoom.adjust_zoom(
                            -*dy * 8.0 * 0.01,
                            mouse_pos,
                            Self::ZOOM_LEVEL_MIN,
                            Self::ZOOM_LEVEL_MAX,
                        );
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        self.editor_state.pan_zoom.adjust_zoom(
                            -pos.y as f32 * 0.01,
                            mouse_pos,
                            Self::ZOOM_LEVEL_MIN,
                            Self::ZOOM_LEVEL_MAX,
                        );
                    }
                }
            }
            _ => {}
        }

        self.egui_winit_state.on_event(&self.egui_context, &event);
    }

    pub fn resize_platform(&mut self, parent_scale: f32, viewport_rect: egui::Rect) {
        // We craft a fake resize event so that the code in egui_winit
        // remains unchanged, thinking it lives in a real window. The poor thing!
        let fake_resize_event = winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize::new(
            (viewport_rect.width() * self.zoom_level() * parent_scale) as u32,
            (viewport_rect.height() * self.zoom_level() * parent_scale) as u32,
        ));

        self.egui_winit_state
            .on_event(&self.egui_context, &fake_resize_event);
    }

    pub fn update(
        &mut self,
        window: &winit::window::Window,
        parent_scale: f32,
        viewport_rect: egui::Rect,
        node_definitions: &NodeDefinitions,
    ) {
        self.resize_platform(parent_scale, viewport_rect);
        self.egui_context.input_mut().pixels_per_point = 1.0 / self.zoom_level();

        // The version with forked egui_winit had the following code:
        // self.egui_context
        //     .begin_frame(
        //         self.egui_winit_state
        //             .take_egui_input(egui_winit::WindowOrSize::Size(egui::vec2(
        //                 viewport_rect.width() * parent_scale * self.zoom_level(),
        //                 viewport_rect.height() * parent_scale * self.zoom_level(),
        //             ))),
        //     );
        // As take_egui_input just transfers inner fields to the return value
        // and uses window only for calculating `screen_rect`,
        // we can call take_egui_input(window) and then modify `screen_rect` manually

        let mut egui_input = self.egui_winit_state.take_egui_input(window);
        let pixels_per_point = self.egui_winit_state.pixels_per_point();
        let screen_size_in_pixels = egui::vec2(
            viewport_rect.width() * parent_scale * self.zoom_level(),
            viewport_rect.height() * parent_scale * self.zoom_level(),
        );
        let screen_size_in_points = screen_size_in_pixels / pixels_per_point;
        egui_input.screen_rect = if screen_size_in_points.x > 0.0 && screen_size_in_points.y > 0.0 {
            Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                screen_size_in_points,
            ))
        } else {
            None
        };

        self.egui_context.begin_frame(egui_input);

        graph::draw_node_graph(
            &self.egui_context,
            &mut self.editor_state,
            &mut self.custom_state,
            node_definitions,
        );

        // Debug mouse pointer position
        // -- This is useful when mouse events are not being interpreted correctly.
        /*
        if let Some(pos) = ctx.input().pointer.hover_pos() {
            ctx.debug_painter()
                .circle(pos, 5.0, egui::Color32::GREEN, egui::Stroke::none());
        } */
    }

    pub fn screen_descriptor(
        &self,
        viewport_rect: egui::Rect,
        parent_scale: f32,
    ) -> ScreenDescriptor {
        ScreenDescriptor {
            size_in_pixels: [
                (viewport_rect.width() * parent_scale) as u32,
                (viewport_rect.height() * parent_scale) as u32,
            ],
            pixels_per_point: 1.0 / self.zoom_level(),
        }
    }

    pub fn add_graph_egui_to_graph<'node>(
        &'node mut self,
        graph: &mut r3::RenderGraph<'node>,
        viewport_rect: egui::Rect,
        parent_scale: f32,
        _resolution: UVec2,
        render_target: r3::RenderTargetHandle,
    ) {
        let full_output = self.egui_context.end_frame();
        let paint_jobs = self.egui_context.tessellate(full_output.shapes);

        let mut builder = graph.add_node("GraphEditorEgui");

        let output_handle = builder.add_render_target_output(render_target);
        let rpass_handle = builder.add_renderpass(r3::RenderPassTargets {
            targets: vec![r3::RenderPassTarget {
                color: output_handle,
                clear: wgpu::Color::BLACK,
                resolve: None,
            }],
            depth_stencil: None,
        });

        let textures_to_free =
            std::mem::replace(&mut self.textures_to_free, full_output.textures_delta.free);
        let self_pt = builder.passthrough_ref_mut(self);

        builder.build(
            move |pt, renderer, encoder_or_pass, _temps, _ready, _graph_data| {
                let this = pt.get_mut(self_pt);
                let rpass = encoder_or_pass.get_rpass(rpass_handle);

                let screen_descriptor = this.screen_descriptor(viewport_rect, parent_scale);

                for tex in textures_to_free {
                    this.renderpass.free_texture(&tex);
                }
                for (id, image_delta) in full_output.textures_delta.set {
                    this.renderpass.update_texture(
                        &renderer.device,
                        &renderer.queue,
                        id,
                        &image_delta,
                    );
                }

                this.renderpass.update_buffers(
                    &renderer.device,
                    &renderer.queue,
                    &paint_jobs,
                    &screen_descriptor,
                );

                this.renderpass
                    .execute_with_renderpass(rpass, &paint_jobs, &screen_descriptor);
            },
        );
    }

    /// Returns Some(render_target) when the graph should be drawn by the parent
    /// context.
    pub fn add_draw_to_graph<'node>(
        &'node mut self,
        graph: &mut r3::RenderGraph<'node>,
        viewport_rect: egui::Rect,
        parent_scale: f32,
    ) -> Option<r3::RenderTargetHandle> {
        let resolution = viewport_rect.size() * parent_scale;
        let resolution = UVec2::new(resolution.x as u32, resolution.y as u32);

        if resolution.x == 0 || resolution.y == 0 {
            return None;
        }

        let render_target = graph.add_render_target(r3::RenderTargetDescriptor {
            label: None,
            resolution,
            samples: r3::SampleCount::One,
            format: r3::TextureFormat::Bgra8UnormSrgb,
            usage: r3::TextureUsages::RENDER_ATTACHMENT | r3::TextureUsages::TEXTURE_BINDING,
        });

        // TODO: Add graph background

        self.add_graph_egui_to_graph(
            graph,
            viewport_rect,
            parent_scale,
            resolution,
            render_target,
        );

        Some(render_target)
    }
}
