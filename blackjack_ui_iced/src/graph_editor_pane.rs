use blackjack_engine::graph::BjkGraph;

use crate::prelude::iced_prelude::*;
use crate::prelude::*;

pub mod node_widget;
pub mod port_widget;

#[derive(Default)]
pub struct GraphEditorPane {}

pub enum GraphEditorMessage {}

impl GraphEditorPane {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, _graph: &mut BjkGraph, message: GraphEditorMessage) {
        match message {}
    }

    pub fn titlebar_view(&self, _graph: &BjkGraph) -> BjkUiElement<'_> {
        row(vec![
            text("Graph editor").into(),
            h_spacer().into(),
            button("Close").into(),
        ])
        .into()
    }

    pub fn content_view(&self, _graph: &BjkGraph) -> BjkUiElement<'_> {
        let node = node_widget::NodeWidget {
            titlebar_left: container(text("This is a node")).padding(4).into(),
            titlebar_right: container(text("x")).padding(4).into(),
            rows: vec![
                node_widget::NodeRow::input(button("B1"), Color::from_rgb8(255, 0, 0)),
                node_widget::NodeRow::input(button("B2"), Color::from_rgb8(0, 255, 0)),
                node_widget::NodeRow::output(button("B3"), Color::from_rgb8(0, 0, 255)),
            ],
            bottom_ui: button("Set Active Node AAA").into(),
            v_separation: 4.0,
            h_separation: 12.0,
            extra_v_separation: 3.0,
        };
        let node = iced::Element::new(node);
        container(node).padding(10).into()
    }
}
