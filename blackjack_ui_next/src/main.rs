use blackjack_engine::{
    graph::{BjkGraph, BjkNodeId, NodeDefinitions},
    lua_engine::LuaRuntime,
};
use egui_wgpu::{winit::Painter, WgpuConfiguration};

use epaint::{
    ahash::{HashMap, HashMapExt},
    Rounding,
};
use guee::{base_widgets::split_pane_container::SplitPaneContainerStyle, prelude::*};
use node_editor_widget::PanZoom;
use slotmap::SecondaryMap;
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{node_editor_widget::NodeEditorWidget, node_widget::NodeWidget};

pub mod node_editor_widget;

pub mod node_widget;

pub struct AppState {
    lua_runtime: LuaRuntime,
    graph_pan_zoom: PanZoom,
    graph: BjkGraph,
    node_positions: SecondaryMap<BjkNodeId, Vec2>,
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

fn test_state() -> AppState {
    // TODO: Hardcoded path
    let runtime = LuaRuntime::initialize_with_std("./blackjack_lua/".into())
        .expect("Lua init should not fail");
    let mut graph = BjkGraph::new();
    let mut node_positions = SecondaryMap::new();

    let node = graph
        .spawn_node("MakeBox", &runtime.node_definitions)
        .unwrap();
    node_positions.insert(node, Vec2::new(40.0, 50.0));

    let node = graph
        .spawn_node("MakeCircle", &runtime.node_definitions)
        .unwrap();
    node_positions.insert(node, Vec2::new(300.0, 150.0));

    AppState {
        lua_runtime: runtime,
        node_positions,
        graph_pan_zoom: PanZoom::default(),
        graph,
    }
}

fn view(state: &AppState) -> DynWidget {
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

    let node_widgets = state.graph.nodes.iter().map(|(node_id, node)| {
        (
            state.node_positions[node_id],
            NodeWidget::from_bjk_node(node_id, node),
        )
    });

    let node_editor = NodeEditorWidget::new(
        IdGen::key("node_editor"),
        node_widgets.collect(),
        state.graph_pan_zoom,
    )
    .on_pan_zoom_change(|state: &mut AppState, new_pan_zoom| {
        state.graph_pan_zoom = new_pan_zoom;
    })
    .build();

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
                        vec![(Vec2::ZERO, panel("bottom")), (Vec2::ZERO, node_editor)],
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

    let mut state = test_state();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
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
