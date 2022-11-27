use blackjack_engine::graph::BjkGraph;
use iced::{executor, Application, Command, Settings};
use root_panes::RootPanesMessage;
use theme::BjkUiTheme;

pub mod graph_editor_pane;
pub mod prelude;
pub mod extensions;
pub mod root_panes;
pub mod theme;

use prelude::*;

#[derive(Debug, Clone)]
pub enum BjkUiMessage {
    RootPanes(RootPanesMessage),
    Dummy,
}

pub enum BlackjackPane {
    GraphEditor,
    Viewport3d,
    Inspector,
}

struct BlackjackUiApp {
    root_panes: root_panes::RootPanes,
    graph: BjkGraph,
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
                graph: BjkGraph::default(),
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
            self.root_panes.view(&self.graph),
        ]))
        .into()
    }
}

fn main() {
    BlackjackUiApp::run(Settings {
        default_font: Some(include_bytes!("../resources/fonts/NunitoSans-Light.ttf")),
        default_text_size: BjkUiTheme::DEFAULT_TEXT_SIZE,
        antialiasing: true,
        ..Default::default()
    }).unwrap();
}
