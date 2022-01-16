use crate::{
    graph::graph_editor_egui::editor_state::EditorState,
    mesh::debug_viz::{self, DebugMeshes},
    prelude::graph::NodeId,
    prelude::*,
};
use std::time::{Instant, Duration};

use egui_winit_platform::Platform;
use winit::{
    dpi::PhysicalSize,
    event::{Event, MouseButton, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

pub mod default_scene;
pub mod gui_overlay;
pub mod input;

use crate::render_context::RenderContext;

use self::input::InputSystem;

struct OrbitCamera {
    yaw: f32,
    pitch: f32,
    distance: f32,
}
impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: -30.0,
            pitch: 30.0,
            distance: 8.0,
        }
    }
}

pub struct AppWindow {
    window: Window,
    scale_factor: f32,
    event_loop: EventLoop<()>,
    state: AppState,
    egui_platform: Platform,
}

pub struct AppState {
    window_size: Vec2,
    input_system: InputSystem,
    orbit_camera: OrbitCamera,
    debug_meshes: Option<DebugMeshes>,
    mesh: Option<HalfEdgeMesh>,
    editor_state: EditorState,
}

impl AppWindow {
    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn new() -> Self {
        let event_loop = winit::event_loop::EventLoop::new();
        let window = {
            let mut builder = winit::window::WindowBuilder::new();
            builder = builder.with_title("My Window");
            builder.build(&event_loop).expect("Could not build window")
        };

        let window_size = window.inner_size();
        let scale_factor = window.scale_factor();

        let egui_platform = Platform::new(egui_winit_platform::PlatformDescriptor {
            physical_width: window_size.width as u32,
            physical_height: window_size.height as u32,
            scale_factor,
            font_definitions: egui::FontDefinitions::default(),
            style: Default::default(),
        });

        let mut editor_state = EditorState::new();

        // A path passed in from the command line will be loaded as a file
        let args : Vec<String> = std::env::args().collect();
        editor_state.load_op = args.get(1).cloned();

        AppWindow {
            scale_factor: window.scale_factor() as f32,
            state: AppState {
                window_size: Vec2::new(window_size.width as f32, window_size.height as f32),
                input_system: InputSystem::default(),
                orbit_camera: OrbitCamera::default(),
                debug_meshes: None,
                mesh: None,
                editor_state,
            },
            event_loop,
            window,
            egui_platform,
        }
    }

    pub fn on_resize(
        render_ctx: &mut RenderContext,
        scale_factor: f32,
        new_size: PhysicalSize<u32>,
    ) {
        render_ctx.on_resize(new_size.width, new_size.height, scale_factor);
    }

    fn update_camera(
        input: &mut InputSystem,
        camera: &mut OrbitCamera,
        render_ctx: &mut RenderContext,
        window_size: Vec2,
    ) {
        // Ignore mouse when it's not on the viewport
        if let Some(pos) = input.mouse.position() {
            if pos.y > window_size.y * 0.5 {
                return;
            }
        }

        // Update status
        if input.mouse.buttons().pressed(MouseButton::Left) {
            camera.yaw += input.mouse.cursor_delta().x * 0.4;
            camera.pitch += input.mouse.cursor_delta().y * 0.4;
        }
        camera.distance += input.mouse.wheel_delta();

        // Compute view matrix
        let view = Mat4::from_translation(Vec3::Z * camera.distance)
            * Mat4::from_rotation_x(-camera.pitch.to_radians())
            * Mat4::from_rotation_y(-camera.yaw.to_radians());
        render_ctx.set_camera(view);
    }

    fn compile_and_run_side_effect(state: &mut AppState, node: NodeId) -> Result<()> {
        let program = crate::graph::graph_compiler::compile_graph(&state.editor_state.graph, node)?;
        program.execute()?;
        Ok(())
    }

    fn compile_and_execute_program(
        state: &mut AppState,
        render_ctx: &mut RenderContext,
    ) -> Result<()> {
        let active = state
            .editor_state
            .active_node
            .ok_or(anyhow!("No active node"))?;
        let program =
            crate::graph::graph_compiler::compile_graph(&state.editor_state.graph, active)?;
        let mesh = program.execute()?;
        let r3mesh = default_scene::build_mesh(&mesh);
        debug_viz::add_halfedge_debug(render_ctx, &mut state.debug_meshes.as_mut().unwrap(), &mesh);
        state.mesh = Some(mesh);
        render_ctx.add_mesh_as_object(r3mesh);
        Ok(())
    }

    fn on_main_events_cleared(
        egui_platform: &mut Platform,
        state: &mut AppState,
        render_ctx: &mut RenderContext,
    ) {
        // Record the frame time at the start of the frame.
        let frame_start_time = Instant::now();

        Self::update_camera(
            &mut state.input_system,
            &mut state.orbit_camera,
            render_ctx,
            state.window_size,
        );
        state.input_system.update();

        egui_platform.begin_frame();
        /*
        gui_overlay::draw_gui_overlays(
            render_ctx,
            state.window_size,
            &egui_platform.context(),
            &state.mesh.as_ref().unwrap(),
        ); */

        render_ctx.clear_objects();

        crate::graph::graph_editor_egui::draw_app(
            &egui_platform.context(),
            &mut state.editor_state,
        );

        if let Some(side_effect) = state.editor_state.run_side_effect.take() {
            Self::compile_and_run_side_effect(state, side_effect)
                .unwrap_or_else(|err| println!("Error when executing node: {}", err));
        }

        let execution_result = Self::compile_and_execute_program(state, render_ctx);

        if let Err(err) = execution_result {
            let painter = egui_platform.context().debug_painter();
            let width = egui_platform.context().available_rect().width();
            painter.text(
                egui::pos2(width - 10.0, 30.0),
                egui::Align2::RIGHT_TOP,
                format!("{}", err),
                egui::TextStyle::Body,
                egui::Color32::RED,
            );
        }

        render_ctx.render_frame(Some(egui_platform), state.window_size.as_uvec2());

        // Sleep for the remaining time to cap at 60Hz
        let elapsed = Instant::now().duration_since(frame_start_time);
        let remaining = Duration::from_secs_f32(1.0 / 60.0).saturating_sub(elapsed);
        spin_sleep::sleep(remaining);
    }

    pub fn setup(&mut self, render_ctx: &mut RenderContext) {
        let mut debug_meshes = debug_viz::add_debug_meshes(&render_ctx.renderer);
        default_scene::add_default_scene(render_ctx, &mut debug_meshes);
        self.state.debug_meshes = Some(debug_meshes);
    }

    pub fn run_app(mut self, mut render_ctx: RenderContext) {
        self.setup(&mut render_ctx);
        self.event_loop.run(move |event, _, control| {
            // The egui platform needs to handle *all* events
            self.egui_platform.handle_event(&event);
            match event {
                Event::WindowEvent { event, .. } => {
                    // NOTE: Several events are forwarded to other subsystems here.
                    self.state.input_system.on_window_event(&event);

                    match event {
                        // Close requested
                        WindowEvent::CloseRequested => {
                            println!("Close requested");
                            *control = winit::event_loop::ControlFlow::Exit;
                        }

                        // Resize
                        WindowEvent::Resized(size) => {
                            self.state.window_size =
                                Vec2::new(size.width as f32, size.height as f32);
                            Self::on_resize(&mut render_ctx, self.scale_factor, size)
                        }

                        _ => {}
                    }
                }
                // Main events cleared
                Event::MainEventsCleared => Self::on_main_events_cleared(
                    &mut self.egui_platform,
                    &mut self.state,
                    &mut render_ctx,
                ),
                _ => {}
            }
        });
    }
}
