use egui_wgpu::{winit::Painter, WgpuConfiguration};

use graph_editor::GraphEditor;
use guee::{
    base_widgets::split_pane_container::SplitPaneContainerStyle, painter::ExtraFont, prelude::*,
};
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub mod widgets;

pub mod graph_editor;

pub struct AppState {
    graph_editor: GraphEditor,
}

pub struct BlackjackPallette {
    pub widget_bg: Color32,
    pub widget_bg_light: Color32,
    pub widget_bg_dark: Color32,

    pub widget_fg: Color32,
    pub widget_fg_light: Color32,
    pub widget_fg_dark: Color32,

    pub accent: Color32,

    pub background: Color32,
    pub background_dark: Color32,
}

#[inline]
fn pallette() -> BlackjackPallette {
    BlackjackPallette {
        widget_bg: color!("#303030"),
        widget_bg_light: color!("#464646"),
        widget_bg_dark: color!("#2c2c2c"),

        widget_fg: color!("#c0c0c0"),
        widget_fg_light: color!("#dddddd"),
        widget_fg_dark: color!("#9b9b9b"),

        accent: color!("#b43e3e"),

        background: color!("#191919"),
        background_dark: color!("#1d1d1d"),
    }
}

pub fn blackjack_theme() -> Theme {
    let mut theme = Theme::new_empty();
    let pallette = pallette();
    theme.set_style::<Button>(ButtonStyle::with_base_colors(
        pallette.widget_bg,
        Stroke::NONE,
        1.1,
        1.3,
    ));

    theme.set_style::<SplitPaneContainer>(SplitPaneContainerStyle::new(pallette.widget_fg_dark));

    theme.text_color = pallette.widget_fg;

    theme
}

fn view(state: &AppState) -> DynWidget {
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
                        panel("left"),
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
    let screen_size = Vec2::new(1024.0, 768.0);
    let mut ctx = Context::new(
        screen_size,
        vec![ExtraFont {
            font_family: epaint::FontFamily::Proportional,
            name: "NunitoSans-Regular",
            data: include_bytes!("../resources/fonts/NunitoSans-Regular.ttf"),
        }],
    );
    ctx.accessor_registry
        .register_accessor(|state: &mut AppState| &mut state.graph_editor);
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

    let mut state = AppState {
        graph_editor: GraphEditor::new(),
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            winit::event::Event::MainEventsCleared => {
                ctx.run(&mut view(&state), &mut state);
                let clipped_primitives = ctx.tessellate();

                let mut textures_delta = TexturesDelta::default();
                if let Some(img_delta) = ctx.painter.borrow().fonts.font_image_delta() {
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
