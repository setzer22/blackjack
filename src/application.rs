use std::{rc::Rc, sync::Arc};

use crate::{prelude::*, rendergraph::grid_routine::GridRoutine};
use egui::{FontDefinitions, Style};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

use self::{
    application_context::ApplicationContext, graph_editor::GraphEditor, root_ui::AppRootAction,
    viewport_3d::Viewport3d, app_viewport::AppViewport,
};

pub struct RootViewport {
    platform: Platform,
    screen_descriptor: ScreenDescriptor,
    renderpass: RenderPass,
    app_context: application_context::ApplicationContext,
    graph_editor: GraphEditor,
    viewport_3d: Viewport3d,
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
            app_context: ApplicationContext::new(renderer),
            graph_editor: GraphEditor::new(&renderer.device, window_size, screen_format),
            viewport_3d: Viewport3d::new(),
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
                event_static.clone(),
            );
            self.viewport_3d.on_winit_event(
                parent_scale,
                self.offscreen_viewports[&OffscreenViewport::Viewport3d].rect,
                event_static.clone(),
            )
        }
    }

    pub fn setup(&mut self, render_ctx: &mut RenderContext) {
        self.app_context.setup(render_ctx);

        let args: Vec<String> = std::env::args().collect();
        if let Some(load_path) = args.get(1) {
            self.handle_root_action(AppRootAction::Load(std::path::PathBuf::from(load_path)))
                .expect("Error loading scene from cli arg");
        }
    }

    pub fn update(&mut self, render_ctx: &mut RenderContext) {
        let mut actions = vec![];

        self.graph_editor.update(
            self.screen_descriptor.scale_factor,
            self.offscreen_viewports[&OffscreenViewport::GraphEditor].rect,
        );
        self.viewport_3d.update(
            self.screen_descriptor.scale_factor,
            self.offscreen_viewports[&OffscreenViewport::Viewport3d].rect,
            render_ctx,
        );

        self.platform.begin_frame();

        egui::TopBottomPanel::top("top_menubar").show(&self.platform.context(), |ui| {
            if let Some(menubar_action) = Self::top_menubar(ui) {
                actions.push(menubar_action);
            }
        });

        egui::CentralPanel::default().show(&self.platform.context(), |ui| {
            let mut split_tree = self.app_context.split_tree.clone();
            split_tree.show(ui, self, Self::show_leaf);
            self.app_context.split_tree = split_tree;
        });

        self.app_context.update(
            &self.platform.context(),
            &mut self.graph_editor.state,
            render_ctx,
        );

        for action in actions {
            // TODO: Don't panic, report error to user in modal dialog
            self.handle_root_action(action)
                .expect("Error executing action");
        }
    }

    pub fn handle_root_action(&mut self, action: AppRootAction) -> Result<()> {
        match action {
            AppRootAction::Save(path) => {
                serialization::save(&self.graph_editor.state, path)?;
                Ok(())
            }
            AppRootAction::Load(path) => {
                self.graph_editor.state = serialization::load(path)?;
                Ok(())
            }
        }
    }

    pub fn render(&mut self, render_ctx: &mut RenderContext) {
        let RenderContext {
            ref base_graph,
            ref pbr_routine,
            ref tonemapping_routine,
            ref grid_routine,
            ..
        } = render_ctx;

        let frame = rend3::util::output::OutputFrame::Surface {
            surface: Arc::clone(&render_ctx.surface),
        };
        let (cmd_bufs, ready) = render_ctx.renderer.ready();
        let mut graph = rend3::graph::RenderGraph::new();
        self.add_root_to_graph(
            &mut graph,
            &ready,
            ViewportRoutines {
                base_graph,
                pbr_routine,
                tonemapping_routine,
                grid_routine,
            },
        );
        graph.execute(&render_ctx.renderer, frame, cmd_bufs, &ready);
    }
}

pub struct ViewportRoutines<'a> {
    base_graph: &'a r3::BaseRenderGraph,
    pbr_routine: &'a r3::PbrRoutine,
    tonemapping_routine: &'a r3::TonemappingRoutine,
    grid_routine: &'a GridRoutine,
}
