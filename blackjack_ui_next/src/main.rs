use egui_wgpu::RenderState;
use graph_editor::GraphEditor;
use guee::{callback_accessor::CallbackAccessor, painter::ExtraFont, prelude::*};
use viewport_3d::Viewport3d;
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub mod blackjack_theme;

pub mod widgets;

pub mod graph_editor;

pub mod viewport_3d;

pub mod renderer;

pub struct AppState {
    graph_editor: GraphEditor,
    viewport_3d: Viewport3d,
}

impl AppState {
    pub fn init() -> Self {
        let cba = CallbackAccessor::<Self>::root();
        Self {
            graph_editor: GraphEditor::new(cba.drill_down(|this| &mut this.graph_editor)),
            viewport_3d: Viewport3d::new(todo!("Bet you'll forget about this one")),
        }
    }
}

fn root_view(state: &AppState, ctx: &Context, render_ctx: &RenderState) -> DynWidget {
    fn panel(key: &str) -> DynWidget {
        ColoredBox::new(IdGen::key(key))
            .hints(LayoutHints::fill())
            .fill(color!("#191919"))
            .stroke(Stroke::new(1.0, color!("#9b9b9b")))
            .build()
    }

    StackContainer::new(
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
                                (Vec2::ZERO, state.viewport_3d.view(render_ctx)),
                            ],
                        ).build(),
                        panel("right"),
                    )
                    .build(),
                    StackContainer::new(
                        IdGen::key("bot_stack"),
                        vec![
                            (Vec2::ZERO, panel("bottom")),
                            (Vec2::ZERO, state.graph_editor.view()),
                        ],
                    )
                    .build(),
                )
                .build(),
            ),
        ],
    )
    .build()
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

    let mut state = AppState::init();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            winit::event::Event::MainEventsCleared => {
                // Run the main view code and generate the root widget
                let mut root_widget =
                    root_view(&state, &ctx, wgpu_painter.render_state().as_ref().unwrap());

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
