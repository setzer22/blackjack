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
    MarginContainer::new(
        IdGen::key("margin"),
        BoxContainer::vertical(
            IdGen::key("vbox"),
            vec![
                BoxContainer::vertical(
                    IdGen::key("items"),
                    state
                        .items
                        .iter()
                        .map(|it| Text::new(it.clone()).build())
                        .collect_vec(),
                )
                .layout_hints(LayoutHints::fill_horizontal())
                .cross_align(Align::Center)
                .build(),
                Spacer::fill_v(1).build(),
                TextEdit::new(
                    IdGen::literal("text_input_field"),
                    state.wip_item_name.clone(),
                )
                .layout_hints(LayoutHints::fill_horizontal())
                .padding(Vec2::new(3.0, 3.0))
                .on_changed(|state: &mut AppState, new| {
                    state.wip_item_name = new;
                })
                .build(),
                BoxContainer::horizontal(
                    IdGen::key("buttons"),
                    vec![
                        Button::with_label("Add!")
                            .on_click(|state: &mut AppState, _| {
                                if !state.wip_item_name.is_empty() {
                                    state.items.push(std::mem::take(&mut state.wip_item_name));
                                }
                            })
                            .hints(LayoutHints::fill_horizontal())
                            .build(),
                        Button::with_label("Delete!")
                            .on_click(|state: &mut AppState, _| {
                                state.items.pop();
                            })
                            .hints(LayoutHints::fill_horizontal())
                            .build(),
                    ],
                )
                .layout_hints(LayoutHints::fill_horizontal())
                .build(),
            ],
        )
        .layout_hints(LayoutHints::fill())
        .build(),
    )
    .margin(Vec2::new(50.0, 50.0))
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
                    epaint::Rgba::from_rgb(0.7, 0.3, 0.3),
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
