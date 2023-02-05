use egui_wgpu::{winit::Painter, WgpuConfiguration};
use itertools::Itertools;

use guee::prelude::*;
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[derive(Default)]
pub struct AppState {
    items: Vec<String>,
    wip_item_name: String,
}

fn view(state: &AppState) -> DynWidget {
    StackContainer::new(
        IdGen::key("stack"),
        vec![
            // Background
            (
                Vec2::new(0.0, 0.0),
                ColoredBox::background(Color32::BLACK).build(),
            ),
            (
                Vec2::new(0.0, 0.0),
                SplitPaneContainer::new(
                    IdGen::key("h_split"),
                    Axis::Horizontal,
                    ColoredBox::background(Color32::RED).build(),
                    ColoredBox::background(Color32::DARK_BLUE).build(),
                )
                .build(),
            ),
        ],
    )
    .build()
}

fn main() {
    let screen_size = Vec2::new(1024.0, 768.0);
    let mut ctx = Context::new(screen_size);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Blackjack")
        .with_inner_size(winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(
            screen_size.x as _,
            screen_size.y as _,
        )))
        .build(&event_loop)
        .unwrap();

    let mut painter = Painter::new(WgpuConfiguration::default(), 1, 0);
    unsafe { pollster::block_on(painter.set_window(Some(&window))).unwrap() };

    let mut state = AppState::default();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            winit::event::Event::MainEventsCleared => {
                ctx.run(&mut view(&state), &mut state);
                let clipped_primitives = ctx.tessellate();

                let mut textures_delta = TexturesDelta::default();
                if let Some(img_delta) = ctx.fonts.font_image_delta() {
                    textures_delta.set.push((TextureId::default(), img_delta));
                }
                painter.paint_and_update_textures(
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
                        painter.on_window_resized(new_size.width, new_size.height);
                    }
                    _ => (),
                }

                ctx.input_state.on_winit_event(&event);
            }
            _ => (),
        }
    })
}
