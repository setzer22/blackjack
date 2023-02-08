use egui_wgpu::{winit::Painter, WgpuConfiguration};

use guee::{base_widgets::split_pane_container::SplitPaneContainerStyle, prelude::*};
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[derive(Default)]
pub struct AppState {}

#[allow(unused)]

pub fn blackjack_theme() -> Theme {
    let mut theme = Theme::new_empty();

    let widget_bg = color!("#303030");
    let widget_bg_light = color!("#464646");
    let widget_bg_dark = color!("#2c2c2c");

    let widget_fg = color!("#c0c0c0");
    let widget_fg_light = color!("#dddddd");
    let widget_fg_dark = color!("#9b9b9b");

    let accent = color!("#b43e3e");

    let background = color!("#191919");
    let background_dark = color!("#1d1d1d");

    theme.set_style::<Button>(ButtonStyle::with_base_colors(
        widget_bg,
        Stroke::NONE,
        1.1,
        1.3,
    ));

    theme.set_style::<SplitPaneContainer>(SplitPaneContainerStyle::new(widget_fg_dark));

    theme.text_color = widget_fg;

    theme
}

fn view(_state: &AppState) -> DynWidget {
    fn panel(key: &str) -> DynWidget {
        MarginContainer::new(
            IdGen::key("margin"),
            ColoredBox::new(IdGen::key(key))
                .hints(LayoutHints::fill())
                .fill(color!("#191919"))
                .stroke(Stroke::new(1.0, color!("#9b9b9b")))
                .build(),
        )
        .margin(Vec2::new(10.0, 10.0))
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
                        panel("left"),
                        panel("right"),
                    )
                    .build(),
                    panel("bottom"),
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
    ctx.set_theme(blackjack_theme());

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
