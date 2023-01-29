// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    app_window::input::viewport_relative_position,
    prelude::{
        graph::{data_type_to_input_param_kind, default_shown_inline, DataTypeUi, ValueTypeUi},
        *,
    },
};
use blackjack_engine::graph::{
    serialization::SerializedBjkSnippet, BlackjackValue, DataType, NodeDefinitions,
};
use egui_wgpu::renderer::{RenderPass, ScreenDescriptor};

use super::{blackjack_theme, gizmo_ui::UiNodeGizmoStates};

pub struct GraphEditor {
    pub editor_state: graph::GraphEditorState,
    pub custom_state: graph::CustomGraphState,
    pub egui_context: egui::Context,
    pub egui_winit_state: egui_winit::State,
    pub renderpass: RenderPass,
    pub raw_mouse_position: Option<egui::Pos2>,
    pub textures_to_free: Vec<egui::TextureId>,
    /// Is the mouse over the node finder? Used to ignore scroll wheel events
    /// and not zoom the graph when that happens.
    pub mouse_over_node_finder: bool,
    /// Stores the last stored contents of the clipboard from the graph editor.
    /// Used to detect whether the current paste event was originated from this
    /// blackjack instance.
    pub previous_clipboard_contents: String,
    /// When there's a potentially unsafe paste operation pending, the parsed
    /// clipboard contents are stored here.
    pub pending_paste_operation: Option<SerializedBjkSnippet>,
    /// Allows ignoring the potentially unsafe paste confirmation dialog.
    pub skip_pending_paste_check: bool,
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
        gizmo_states: UiNodeGizmoStates,
    ) -> Self {
        let egui_context = egui::Context::default();
        egui_context.set_visuals(blackjack_graph_theme());

        let mut egui_winit_state = egui_winit::State::new_with_wayland_display(None);
        egui_winit_state.set_max_texture_side(renderer.limits.max_texture_dimension_2d as usize);
        egui_winit_state.set_pixels_per_point(1.0);

        Self {
            // Set default zoom to the inverse of ui scale to preserve dpi
            editor_state: graph::GraphEditorState::new(1.0 / parent_scale),
            custom_state: graph::CustomGraphState::new(node_definitions, gizmo_states),
            egui_context,
            egui_winit_state,
            renderpass: RenderPass::new(&renderer.device, format, 1),
            // The mouse position, in window coordinates. Stored to hide other
            // window events from egui when the cursor is not over the viewport
            raw_mouse_position: None,
            textures_to_free: Vec::new(),
            mouse_over_node_finder: false,
            previous_clipboard_contents: String::new(),
            pending_paste_operation: None,
            skip_pending_paste_check: false,
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
                if !self.mouse_over_node_finder {
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

        graph::draw_node_graph(self);

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
    ) -> egui::PlatformOutput {
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

        full_output.platform_output
    }

    /// Returns Some(render_target) when the graph should be drawn by the parent
    /// context.
    pub fn add_draw_to_graph<'node>(
        &'node mut self,
        graph: &mut r3::RenderGraph<'node>,
        viewport_rect: egui::Rect,
        parent_scale: f32,
    ) -> (Option<r3::RenderTargetHandle>, Option<egui::PlatformOutput>) {
        let resolution = viewport_rect.size() * parent_scale;
        let resolution = UVec2::new(resolution.x as u32, resolution.y as u32);

        if resolution.x == 0 || resolution.y == 0 {
            return (None, None);
        }

        let render_target = graph.add_render_target(r3::RenderTargetDescriptor {
            label: None,
            resolution,
            samples: r3::SampleCount::One,
            format: r3::TextureFormat::Bgra8UnormSrgb,
            usage: r3::TextureUsages::RENDER_ATTACHMENT | r3::TextureUsages::TEXTURE_BINDING,
        });

        // TODO: Add graph background

        let platform_output = self.add_graph_egui_to_graph(
            graph,
            viewport_rect,
            parent_scale,
            resolution,
            render_target,
        );

        (Some(render_target), Some(platform_output))
    }

    /// Updates the graph after the node definitions were updated. This
    /// reconciles the state stored in the graph with any changes in the Lua
    /// code, such as newly added parameters, removed parameters or other kinds
    /// of changes.
    pub fn on_node_definitions_update(&mut self) -> Result<()> {
        let node_defs = self.custom_state.node_definitions.share();
        let graph = &mut self.editor_state.graph;

        use egui_node_graph::{InputId, NodeId, OutputId};
        enum DelayedOps {
            /// The operation for this NodeId is no longer found in the node
            /// definitions.
            RemovedNodeType(NodeId),
            /// The label for this node has changed.
            NodeLabelRenamed { node_id: NodeId, new_label: String },
            /// A new input parameter has been added.
            NewInput {
                node_id: NodeId,
                param_name: String,
                data_type: DataType,
                value: BlackjackValue,
            },
            /// The DataType for an input has changed
            InputChangedType {
                input_id: InputId,
                new_type: DataType,
            },
            /// An input parameter was removed
            InputRemoved { input_id: InputId },
            /// A new output parameter has been added.
            NewOutput {
                node_id: NodeId,
                param_name: String,
                data_type: DataType,
            },
            /// The DataType for an output has changed
            OutputChangedType {
                output_id: OutputId,
                new_type: DataType,
            },
            /// An output parameter was removed
            OutputRemoved { output_id: OutputId },
        }

        let mut delayed_ops = vec![];

        for (node_id, node) in &graph.nodes {
            if let Some(node_def) = node_defs.node_def(&node.user_data.op_name) {
                if node.label != node_def.label {
                    delayed_ops.push(DelayedOps::NodeLabelRenamed {
                        new_label: node_def.label.clone(),
                        node_id,
                    });
                }

                // Handle input parameters
                let mut defined_inputs = HashSet::new();
                for input in &node_def.inputs {
                    defined_inputs.insert(&input.name);

                    if let Some((_, input_id)) = graph[node_id]
                        .inputs
                        .iter()
                        .find(|(i_name, _)| i_name == &input.name)
                    {
                        if graph[*input_id].typ.0 != input.data_type {
                            delayed_ops.push(DelayedOps::InputChangedType {
                                input_id: *input_id,
                                new_type: input.data_type,
                            })
                        }
                    } else {
                        delayed_ops.push(DelayedOps::NewInput {
                            node_id,
                            param_name: input.name.clone(),
                            data_type: input.data_type,
                            value: input.default_value(),
                        })
                    }
                }

                for (input_name, input_id) in &graph[node_id].inputs {
                    if !defined_inputs.contains(&input_name) {
                        delayed_ops.push(DelayedOps::InputRemoved {
                            input_id: *input_id,
                        })
                    }
                }

                // Handle output parameters
                let mut defined_outputs = HashSet::new();
                for output in &node_def.outputs {
                    defined_outputs.insert(&output.name);

                    if let Some((_, output_id)) = graph[node_id]
                        .outputs
                        .iter()
                        .find(|(o_name, _)| o_name == &output.name)
                    {
                        if graph[*output_id].typ.0 != output.data_type {
                            delayed_ops.push(DelayedOps::OutputChangedType {
                                output_id: *output_id,
                                new_type: output.data_type,
                            })
                        }
                    } else {
                        delayed_ops.push(DelayedOps::NewOutput {
                            node_id,
                            param_name: output.name.clone(),
                            data_type: output.data_type,
                        })
                    }
                }

                for (output_name, output_id) in &graph[node_id].outputs {
                    if !defined_outputs.contains(&output_name) {
                        delayed_ops.push(DelayedOps::OutputRemoved {
                            output_id: *output_id,
                        })
                    }
                }
            } else {
                delayed_ops.push(DelayedOps::RemovedNodeType(node_id));
            }
        }

        for op in delayed_ops {
            match op {
                DelayedOps::RemovedNodeType(_) => {
                    // We don't do anything if the node definition does not exist
                    // for this node. Sometimes the user may screw up and remove a
                    // node definition, but we want to keep the old data around for
                    // when they fix their Lua code, otherwise they may blow their
                    // entire graph.
                    //
                    // We still keep this around in case we later want to
                    // implement some corrective action, or throw some kind of
                    // warning.
                }
                DelayedOps::NewInput {
                    node_id,
                    param_name,
                    data_type,
                    value,
                } => {
                    graph.add_input_param(
                        node_id,
                        param_name,
                        DataTypeUi(data_type),
                        ValueTypeUi(value),
                        data_type_to_input_param_kind(data_type),
                        default_shown_inline(),
                    );
                }
                DelayedOps::InputChangedType { input_id, new_type } => {
                    graph.remove_connection(input_id);
                    graph[input_id].typ = DataTypeUi(new_type);
                }
                DelayedOps::InputRemoved { input_id } => {
                    graph.remove_input_param(input_id);
                }
                DelayedOps::NewOutput {
                    node_id,
                    param_name,
                    data_type,
                } => {
                    graph.add_output_param(node_id, param_name, DataTypeUi(data_type));
                }
                DelayedOps::OutputChangedType {
                    output_id,
                    new_type,
                } => {
                    let inputs = graph
                        .connections
                        .iter()
                        .filter(|(_, o)| **o == output_id)
                        .map(|(i, _)| i)
                        .collect_vec();
                    for input in inputs {
                        graph.remove_connection(input);
                    }
                    graph[output_id].typ = DataTypeUi(new_type);
                }
                DelayedOps::OutputRemoved { output_id } => {
                    graph.remove_output_param(output_id);
                }
                DelayedOps::NodeLabelRenamed { node_id, new_label } => {
                    graph[node_id].label = new_label;
                }
            }
        }

        Ok(())
    }
}
