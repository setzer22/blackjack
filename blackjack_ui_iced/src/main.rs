use blackjack_engine::graph::{BjkGraph, BjkNodeId};
use glam::Vec2;
use graph_editor_pane::{GraphEditorPane, GraphEditorState, GraphPaneMessage};
use iced::{executor, Application, Command, Settings};
use root_panes::{BlackjackPane, RootPanesMessage};
use slotmap::SecondaryMap;
use theme::BjkUiTheme;

pub mod extensions;
pub mod graph_editor_pane;
pub mod prelude;
pub mod root_panes;
pub mod theme;

use prelude::*;

#[derive(Debug, Clone)]
pub enum BjkUiMessage {
    RootPanes(RootPanesMessage),
    GraphPane(GraphPaneMessage),
    Dummy,
}

pub struct BlackjackUiApp {
    root_panes: root_panes::RootPanes,
    graph_editor: GraphEditorState,
}

impl Application for BlackjackUiApp {
    type Executor = executor::Default;
    type Message = BjkUiMessage;
    type Theme = BjkUiTheme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            BlackjackUiApp {
                root_panes: root_panes::RootPanes::new(),
                graph_editor: GraphEditorState::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Blackjack".into()
    }

    fn theme(&self) -> Self::Theme {
        Self::Theme::default()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            BjkUiMessage::GraphPane(msg) => {
                self.graph_editor.update(msg);
            }
            BjkUiMessage::RootPanes(msg) => {
                self.root_panes.update(msg);
            }
            BjkUiMessage::Dummy => {}
        }
        Command::none()
    }

    fn view(&self) -> BjkUiElement<'_> {
        container(column(vec![
            text("Blackjack").into(),
            self.root_panes
                .view(self, Self::pane_title, Self::pane_contents),
        ]))
        .into()
    }
}

impl BlackjackUiApp {
    fn pane_title(&self, pane: BlackjackPane) -> BjkUiElement<'_> {
        match pane {
            BlackjackPane::GraphEditor => GraphEditorPane.titlebar_view(&self.graph_editor),
            BlackjackPane::Viewport3d => text("Viewport 3d").into(),
            BlackjackPane::Inspector => text("Inspector").into(),
            BlackjackPane::Spreadsheet => text("Spreadsheet").into(),
        }
    }

    fn pane_contents(&self, pane: BlackjackPane) -> BjkUiElement<'_> {
        match pane {
            BlackjackPane::GraphEditor => GraphEditorPane.content_view(&self.graph_editor),
            BlackjackPane::Viewport3d => text("I am the 3d viewport").into(),
            BlackjackPane::Inspector => text("I am the inspector 🕵").into(),
            BlackjackPane::Spreadsheet => text("I am the mighty spreadsheet").into(),
        }
    }
}

fn main() {
    BlackjackUiApp::run(Settings {
        default_font: Some(include_bytes!("../resources/fonts/NunitoSans-Light.ttf")),
        default_text_size: BjkUiTheme::DEFAULT_TEXT_SIZE,
        antialiasing: true,
        ..Default::default()
    })
    .unwrap();
}
