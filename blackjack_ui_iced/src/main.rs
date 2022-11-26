use blackjack_engine::graph::BjkGraph;
use iced::{
    executor,
    widget::{column, container, text},
    Application, Command, Settings, Theme,
};
use root_panes::RootPanesMessage;

pub mod graph_editor_pane;
pub mod root_panes;

#[derive(Debug, Clone)]
pub enum BlackjackUiMessage {
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
    type Message = BlackjackUiMessage;
    type Theme = Theme;
    type Flags = ();

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
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
            BlackjackUiMessage::RootPanes(msg) => {
                self.root_panes.update(msg);
            }
            Dummy => {}
        }
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        container(column(vec![
            text("Blackjack").into(),
            self.root_panes.view(&self.graph).into(),
        ]))
        .into()
    }
}

fn main() {
    BlackjackUiApp::run(Settings::default()).unwrap();
}
