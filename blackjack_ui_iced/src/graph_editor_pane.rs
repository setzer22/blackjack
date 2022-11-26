use blackjack_engine::graph::BjkGraph;
use iced::{
    widget::{button, column, container, text},
    Padding,
};
use iced_native::widget::row;

pub mod node_widget;

#[derive(Default)]
pub struct GraphEditorPane {}

pub enum GraphEditorMessage {}

impl GraphEditorPane {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, graph: &mut BjkGraph, message: GraphEditorMessage) {
        match message {}
    }

    pub fn titlebar_view(&self, _graph: &BjkGraph) -> iced::Element<'_, super::BlackjackUiMessage> {
        row(vec![
            iced::widget::text("Graph editor").into(),
            iced::widget::Container::new(iced::widget::text(""))
                .width(iced::Length::Fill)
                .into(),
            iced::widget::button("Close").into(),
        ])
        .into()
    }

    pub fn content_view(&self, graph: &BjkGraph) -> iced::Element<'_, super::BlackjackUiMessage> {
        let node = node_widget::NodeWidget {
            titlebar_left: container(text("This is a node")).padding(4).into(),
            titlebar_right: container(text("x")).padding(4).into(),
            rows: vec![
                iced::widget::button("B1").into(),
                iced::widget::button("B2").into(),
                iced::widget::button("B3").into(),
            ],
            bottom_ui: iced::widget::button("Set Active Node AAA").into(),
            v_separation: 4.0,
            h_separation: 12.0,
            extra_v_separation: 3.0,
        };
        let node = iced::Element::new(node);
        container(node).padding(10).into()
    }
}
