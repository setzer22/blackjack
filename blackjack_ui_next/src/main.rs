use std::rc::Rc;
use std::time::{Duration, Instant};

use blackjack_engine::graph_interpreter::run_graph;
use egui_wgpu::RenderState;
use graph_editor::GraphEditor;
use guee::base_widgets::menubar_button::MenubarButton;
use guee::callback_accessor::CallbackAccessor;
use guee::extension_traits::Color32Ext;
use guee::painter::ExtraFont;
use guee::prelude::*;
use icon_management::IconAtlas;
use renderer::texture_manager::TextureManager;
use viewport_3d::Viewport3d;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use crate::blackjack_theme::pallette;

pub mod blackjack_theme;

pub mod widgets;

pub mod graph_editor;

pub mod viewport_3d;

pub mod renderer;

pub mod serialization;

pub mod icon_management;

pub enum RootAction {
    Save,
    Load,
}

pub struct AppState {
    graph_editor: GraphEditor,
    viewport_3d: Viewport3d,
    cba: CallbackAccessor<Self>,
    pending_actions: Vec<RootAction>,
    texture_manager: TextureManager,
    icon_atlas: Rc<IconAtlas>,
}

impl AppState {
    pub fn init(render_ctx: &RenderState) -> Self {
        let cba = CallbackAccessor::<Self>::root();
        let mut texture_manager =
            TextureManager::new(render_ctx.device.clone(), render_ctx.queue.clone());

        let icon_atlas = Rc::new(IconAtlas::new(render_ctx, &mut texture_manager));

        Self {
            graph_editor: GraphEditor::new(
                cba.drill_down(|this| &mut this.graph_editor),
                icon_atlas.clone(),
            ),
            viewport_3d: Viewport3d::new(
                render_ctx,
                cba.drill_down(|this| &mut this.viewport_3d),
                &mut texture_manager,
            ),
            texture_manager,
            pending_actions: Vec::new(),
            icon_atlas,
            cba,
        }
    }
}

impl AppState {
    fn root_view(&self, _ctx: &Context, render_ctx: &RenderState) -> DynWidget {
        fn panel(key: &str) -> DynWidget {
            ColoredBox::new(IdGen::key(key))
                .hints(LayoutHints::fill())
                .fill(color!("#19191933"))
                .stroke(Stroke::new(0.5, color!("#9b9b9b")))
                .build()
        }

        let main_view = StackContainer::new(
            IdGen::key("stack"),
            vec![
                // Background
                (
                    Vec2::new(0.0, 0.0),
                    ColoredBox::background(color!("#1d1d1d")).build(),
                ),
                (
                    Vec2::new(0.0, 0.0),
                    SplitPaneContainer::new(
                        IdGen::key("v_split"),
                        Axis::Vertical,
                        SplitPaneContainer::new(
                            IdGen::key("h_split"),
                            Axis::Horizontal,
                            StackContainer::new(
                                IdGen::key("left_stack"),
                                vec![
                                    (Vec2::ZERO, panel("bottom")),
                                    (
                                        Vec2::ZERO,
                                        self.viewport_3d.view(render_ctx, &self.texture_manager),
                                    ),
                                ],
                            )
                            .build(),
                            panel("right"),
                        )
                        .build(),
                        StackContainer::new(
                            IdGen::key("bot_stack"),
                            vec![
                                (Vec2::ZERO, panel("bottom")),
                                (Vec2::ZERO, self.graph_editor.view()),
                            ],
                        )
                        .build(),
                    )
                    .build(),
                ),
            ],
        )
        .build();

        let menubar_offset = Vec2::new(0.0, 24.0);
        let top_menubar = StackContainer::new(
            IdGen::key("top_menubar"),
            vec![
                (
                    Vec2::ZERO,
                    ColoredBox::background(pallette().widget_bg_dark)
                        .min_size(menubar_offset)
                        .hints(LayoutHints::fill_horizontal())
                        .stroke(Stroke {
                            width: 2.0,
                            color: pallette().widget_bg_dark.lighten(1.1),
                        })
                        .build(),
                ),
                (
                    Vec2::ZERO,
                    BoxContainer::horizontal(
                        IdGen::key("h"),
                        vec![MenubarButton::new(
                            IdGen::key("b"),
                            "File".into(),
                            vec!["Save 'Jack' As".into(), "Open 'Jack'".into()],
                        )
                        .button_icons(vec![
                            self.icon_atlas.get_icon("floppy-disk").unwrap(),
                            self.icon_atlas.get_icon("open-folder").unwrap(),
                        ])
                        .on_option_selected(self.cba.callback(|app_state, idx| match idx {
                            0 => app_state.pending_actions.push(RootAction::Save),
                            1 => app_state.pending_actions.push(RootAction::Load),
                            _ => unreachable!(),
                        }))
                        .menu_min_width(150.0)
                        .build()],
                    )
                    .build(),
                ),
            ],
        )
        .build();

        StackContainer::new(
            // NOTE: Can't use a BoxContainer::vertical here because:
            // - We don't want two-pass layout at the top-level of our view
            // - Some of the top-level widgets don't support being drawin inside a flex container
            IdGen::key("blackjack"),
            vec![(Vec2::ZERO, top_menubar), (menubar_offset, main_view)],
        )
        .build()
    }

    fn update(&mut self, _context: &Context) {
        for root_action in self.pending_actions.drain(..) {
            match root_action {
                RootAction::Save => {
                    let file_location = rfd::FileDialog::new()
                        .set_file_name("Untitled.bjk")
                        .add_filter("Blackjack Models", &["bjk"])
                        .save_file();
                    if let Some(path) = file_location {
                        // TODO: Improve error notifications
                        match serialization::save(&path, &self.graph_editor) {
                            Ok(_) => (),
                            Err(err) => {
                                println!("Error when saving: {err}");
                            }
                        }
                    }
                }
                RootAction::Load => {
                    let file_location = rfd::FileDialog::new()
                        .add_filter("Blackjack Models", &["bjk"])
                        .pick_file();
                    if let Some(path) = file_location {
                        // TODO: Improve error notifications
                        match serialization::load(&path, &mut self.graph_editor) {
                            Ok(_) => (),
                            Err(err) => {
                                println!("Error when loading: {err}");
                            }
                        }
                    }
                }
            }
        }

        let renderable = if let Some(active_node) = self.graph_editor.graph.default_node {
            // TODO: Change detection
            self.graph_editor.external_parameters.fill_defaults(
                &self.graph_editor.graph,
                &self.graph_editor.lua_runtime.node_definitions,
            );
            let program_result = run_graph(
                &self.graph_editor.lua_runtime.lua,
                &self.graph_editor.graph,
                active_node,
                self.graph_editor.external_parameters.clone(),
                &self.graph_editor.lua_runtime.node_definitions,
                None, // TODO: Gizmos
            );

            match program_result {
                Ok(result) => result.renderable,
                Err(err) => {
                    println!("TODO {err}");
                    None
                }
            }
        } else {
            None
        };

        self.viewport_3d.update(renderable);
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let screen_size = Vec2::new(1024.0, 768.0);
    let mut ctx = Context::new(
        screen_size,
        vec![ExtraFont {
            font_family: epaint::FontFamily::Proportional,
            name: "NunitoSans-Regular",
            data: include_bytes!("../resources/fonts/NunitoSans-Regular.ttf"),
        }],
    );
    ctx.set_theme(blackjack_theme::blackjack_theme());

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Blackjack")
        .with_inner_size(winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(
            screen_size.x as _,
            screen_size.y as _,
        )))
        .build(&event_loop)
        .unwrap();

    let mut wgpu_painter =
        egui_wgpu::winit::Painter::new(egui_wgpu::WgpuConfiguration::default(), 1, 0);
    unsafe { pollster::block_on(wgpu_painter.set_window(Some(&window))).unwrap() };

    let mut state = AppState::init(wgpu_painter.render_state().as_ref().unwrap());

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            winit::event::Event::MainEventsCleared => {
                // Record the frame time at the start of the frame.
                let frame_start_time = Instant::now();

                // Run the main view code and generate the root widget
                let mut root_widget =
                    state.root_view(&ctx, wgpu_painter.render_state().as_ref().unwrap());

                // Layout, push shapes and trigger side-effects
                ctx.run(&mut root_widget, &mut state);

                // Tessellate and render the pushed shapes
                let clipped_primitives = ctx.tessellate();
                let mut textures_delta = TexturesDelta::default();
                if let Some(img_delta) = ctx.painter.borrow().fonts.font_image_delta() {
                    textures_delta.set.push((TextureId::default(), img_delta));
                }
                wgpu_painter.paint_and_update_textures(
                    1.0,
                    // Make it very obvious when the background is visible.
                    epaint::Rgba::from_rgb(1.0, 0.0, 1.0),
                    &clipped_primitives,
                    &textures_delta,
                );

                // Run update logic
                state.update(&ctx);

                // Sleep for the remaining time to cap at 60Hz
                let elapsed = Instant::now().duration_since(frame_start_time);
                let remaining = Duration::from_secs_f32(1.0 / 60.0).saturating_sub(elapsed);
                spin_sleep::sleep(remaining);
            }
            winit::event::Event::WindowEvent { window_id, event } if window_id == window.id() => {
                match &event {
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    winit::event::WindowEvent::Resized(new_size) => {
                        wgpu_painter.on_window_resized(new_size.width, new_size.height);
                    }
                    _ => (),
                }

                ctx.on_winit_event(&event);
            }
            _ => (),
        }
    })
}
