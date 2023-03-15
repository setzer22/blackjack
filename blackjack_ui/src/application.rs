// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use crate::{
    cli_args::CLI_ARGS,
    prelude::*,
    rendergraph::{
        face_routine::FaceRoutine, grid_routine::GridRoutine, id_picking_routine::IdPickingRoutine,
        point_cloud_routine::PointCloudRoutine, wireframe_routine::WireframeRoutine,
    },
};
use blackjack_engine::lua_engine::LuaRuntime;
use egui_wgpu::renderer::{RenderPass, ScreenDescriptor};
use winit::window::Window;

use self::{
    app_viewport::AppViewport, application_context::ApplicationContext,
    gizmo_ui::UiNodeGizmoStates, graph_editor::GraphEditor, inspector::InspectorTabs,
    root_ui::AppRootAction, viewport_3d::Viewport3d,
};

pub struct RootViewport {
    egui_winit_state: egui_winit::State,
    egui_context: egui::Context,
    textures_to_free: Vec<egui::TextureId>,
    screen_descriptor: ScreenDescriptor,
    renderpass: RenderPass,
    app_context: application_context::ApplicationContext,
    graph_editor: GraphEditor,
    viewport_3d: Viewport3d,
    /// Stores the egui texture ids for the child viewports.
    offscreen_viewports: HashMap<OffscreenViewport, AppViewport>,
    inspector_tabs: InspectorTabs,
    diagnostics_open: bool,
    lua_runtime: LuaRuntime,
    mouse_captured_by_split: bool,
}

/// The application context is state that is global to an instance of blackjack.
/// The currently open file and any data that is not per-viewport goes here.
pub mod application_context;

/// The gizmo logic specific to blackjack_ui
pub mod gizmo_ui;

/// The graph editor viewport. Shows an inner egui instance with zooming /
/// panning functionality.
pub mod graph_editor;

/// The 3d viewport, shows the current mesh
pub mod viewport_3d;

/// The rend3 portion of the rendergraph for the root viewport
pub mod root_graph;

/// The egui code for the root viewport
pub mod root_ui;

/// Serialization code to load / store graphs
pub mod serialization;

/// An egui widget that draws an offscreen-rendered texture
pub mod app_viewport;

/// An egui container to draw a recursive tree of resizable horizontal/vertical splits
pub mod viewport_split;

/// The properties and spreadsheet inspector code
pub mod inspector;

/// An egui widget to display a text editor with source code and syntax
/// highlighting support
pub mod code_viewer;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum OffscreenViewport {
    GraphEditor,
    Viewport3d,
}

pub fn blackjack_theme() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.widgets.noninteractive.bg_fill = color_from_hex("#303030").unwrap();
    visuals.widgets.noninteractive.fg_stroke.color = color_from_hex("#C0C0C0").unwrap();
    visuals.extreme_bg_color = color_from_hex("#1d1d1d").unwrap();
    visuals.selection.bg_fill = color_from_hex("#b43e3e").unwrap();
    visuals.selection.stroke.color = color_from_hex("#fdfdfd").unwrap();
    visuals
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

        let egui_context = egui::Context::default();
        egui_context.set_visuals(blackjack_theme());

        let mut egui_winit_state = egui_winit::State::new_with_wayland_display(None);
        egui_winit_state.set_max_texture_side(renderer.limits.max_texture_dimension_2d as usize);
        egui_winit_state.set_pixels_per_point(scale_factor as f32);

        // TODO: Hardcoded node libraries path. Read from cmd line?
        let mut lua_runtime = LuaRuntime::initialize_with_std("./blackjack_lua/".into())
            .unwrap_or_else(|err| panic!("Init lua should not fail. {err}"));
        if !CLI_ARGS.disable_lua_watcher {
            lua_runtime
                .start_file_watcher()
                .expect("Error starting file watcher.");
        }

        let gizmo_state = UiNodeGizmoStates::init();
        RootViewport {
            egui_winit_state,
            egui_context,
            textures_to_free: Vec::new(),
            screen_descriptor: ScreenDescriptor {
                size_in_pixels: window_size.to_array(),
                pixels_per_point: scale_factor as f32,
            },
            renderpass: RenderPass::new(&renderer.device, screen_format, 1),
            app_context: ApplicationContext::new(gizmo_state.share()),
            graph_editor: GraphEditor::new(
                renderer,
                screen_format,
                scale_factor as f32,
                lua_runtime.node_definitions.share(),
                gizmo_state.share(),
            ),
            viewport_3d: Viewport3d::new(),
            offscreen_viewports,
            inspector_tabs: InspectorTabs::new(),
            diagnostics_open: false,
            lua_runtime,
            mouse_captured_by_split: false,
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

        #[allow(clippy::single_match)]
        match event {
            winit::event::Event::WindowEvent { ref event, .. } => match event {
                winit::event::WindowEvent::Resized(new_size) => {
                    self.screen_descriptor.size_in_pixels[0] = new_size.width;
                    self.screen_descriptor.size_in_pixels[1] = new_size.height;
                }
                winit::event::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    self.screen_descriptor.pixels_per_point = *scale_factor as f32;
                }
                _ => {}
            },
            _ => {}
        }

        if let winit::event::Event::WindowEvent { event, .. } = event {
            self.egui_winit_state.on_event(&self.egui_context, &event);
            let parent_scale = self.screen_descriptor.pixels_per_point;
            if let Some(event) = event.to_static() {
                self.graph_editor.on_winit_event(
                    parent_scale,
                    self.offscreen_viewports[&OffscreenViewport::GraphEditor].rect,
                    event.clone(),
                    self.mouse_captured_by_split,
                );
                self.viewport_3d.on_winit_event(
                    parent_scale,
                    self.offscreen_viewports[&OffscreenViewport::Viewport3d].rect,
                    event.clone(),
                    self.mouse_captured_by_split,
                )
            }
        }
    }

    pub fn setup(&mut self, render_ctx: &mut RenderContext) {
        self.app_context.setup(render_ctx);

        if let Some(load) = &CLI_ARGS.load {
            self.handle_root_action(AppRootAction::Load(std::path::PathBuf::from(load)))
                .expect("Error loading scene from CLI arg.");
        }
    }

    pub fn update(&mut self, render_ctx: &mut RenderContext, window: &winit::window::Window) {
        let mut actions = vec![];

        if !CLI_ARGS.disable_lua_watcher {
            match self.lua_runtime.watch_for_changes() {
                Ok(true) => {
                    if let Err(err) = self.graph_editor.on_node_definitions_update() {
                        println!("Error while updating graph after Lua code reload: {err}.");
                    }

                    // Reset gizmo state when code is reloaded. This helps
                    // interactively develop gizmos, otherwise the init function
                    // is not run again after reloading.
                    self.app_context.node_gizmo_states.reset_for_hot_reload();
                }
                Ok(false) => { /* Do nothing */ }
                Err(err) => {
                    println!("Error while reloading Lua code: {err}.");
                }
            }
        }

        self.graph_editor.update(
            window,
            self.screen_descriptor.pixels_per_point,
            self.offscreen_viewports[&OffscreenViewport::GraphEditor].rect,
        );
        self.viewport_3d.update(
            self.screen_descriptor.pixels_per_point,
            self.offscreen_viewports[&OffscreenViewport::Viewport3d].rect,
            render_ctx,
        );

        self.egui_context
            .begin_frame(self.egui_winit_state.take_egui_input(window));

        if let Some(menubar_action) = self.top_menubar() {
            actions.push(menubar_action);
        }

        egui::CentralPanel::default().show(&self.egui_context.clone(), |ui| {
            let mut split_tree = self.app_context.split_tree.clone();

            impl viewport_split::PayloadTrait for RootViewport {
                fn notify_interacted(&mut self) {
                    self.mouse_captured_by_split = true;
                }
            }

            self.mouse_captured_by_split = false; // Will be set by `show`.
            split_tree.show(ui, self, Self::show_leaf);
            self.app_context.split_tree = split_tree;
        });

        self.diagnostics_ui();

        actions.extend(self.app_context.update(
            &self.egui_context,
            &mut self.graph_editor.editor_state,
            &mut self.graph_editor.custom_state,
            render_ctx,
            &self.viewport_3d.settings,
            &self.lua_runtime,
        ));

        for action in actions {
            // TODO: Don't panic, report error to user in modal dialog
            self.handle_root_action(action)
                .expect("Error executing action.");
        }
    }

    pub fn handle_root_action(&mut self, action: AppRootAction) -> Result<()> {
        match action {
            AppRootAction::Save(path) => {
                serialization::save(
                    &self.graph_editor.editor_state,
                    &self.graph_editor.custom_state,
                    path,
                )?;
            }
            AppRootAction::Load(path) => {
                let (editor_state, custom_state) = serialization::load(
                    path,
                    &self.graph_editor.custom_state.node_definitions,
                    &self.graph_editor.custom_state.gizmo_states,
                )?;
                self.graph_editor.editor_state = editor_state;
                self.graph_editor.custom_state = custom_state;
            }
        }
        Ok(())
    }

    pub fn render(&mut self, render_ctx: &mut RenderContext) -> egui::PlatformOutput {
        let RenderContext {
            ref base_graph,
            ref pbr_routine,
            ref tonemapping_routine,
            ref grid_routine,
            ref wireframe_routine,
            ref point_cloud_routine,
            ref face_routine,
            ref mut id_picking_routine,
            ..
        } = render_ctx;

        // TODO: Maybe this is not the best place to do this. Do it in `update` instead?
        id_picking_routine.set_cursor_pos(
            self.egui_context
                .input()
                .pointer
                .hover_pos()
                .unwrap_or(egui::Pos2::ZERO),
            self.viewport_3d.viewport_rect(),
        );

        let frame = rend3::util::output::OutputFrame::Surface {
            surface: Arc::clone(&render_ctx.surface),
        };
        let (cmd_bufs, ready) = render_ctx.renderer.ready();
        let mut graph = rend3::graph::RenderGraph::new();
        let platform_output = self.add_root_to_graph(
            &mut graph,
            &ready,
            ViewportRoutines {
                base_graph,
                pbr: pbr_routine,
                tonemapping: tonemapping_routine,
                grid: grid_routine,
                wireframe: wireframe_routine,
                point_cloud: point_cloud_routine,
                face: face_routine,
                id_picking: id_picking_routine,
            },
        );

        // Use error scopes to prevent wgpu validation errors from crashing.
        render_ctx
            .renderer
            .device
            .push_error_scope(wgpu::ErrorFilter::Validation);

        graph.execute(&render_ctx.renderer, frame, cmd_bufs, &ready);

        if let Some(error) = pollster::block_on(render_ctx.renderer.device.pop_error_scope()) {
            println!("Error validating WebGPU: {error}.");
        }

        let id = id_picking_routine.id_under_mouse(&render_ctx.renderer.device);
        self.app_context.on_id_hovered(id);

        platform_output
    }

    pub fn handle_platform_output(
        &mut self,
        window: &Window,
        platform_output: egui::PlatformOutput,
    ) {
        self.egui_winit_state
            .handle_platform_output(window, &self.egui_context, platform_output);
    }
}

pub struct ViewportRoutines<'a> {
    pub base_graph: &'a r3::BaseRenderGraph,
    pub pbr: &'a r3::PbrRoutine,
    pub tonemapping: &'a r3::TonemappingRoutine,
    pub grid: &'a GridRoutine,
    pub wireframe: &'a WireframeRoutine,
    pub point_cloud: &'a PointCloudRoutine,
    pub face: &'a FaceRoutine,
    pub id_picking: &'a IdPickingRoutine,
}
