use iced::{
    executor,
    widget::{button, column, container, pane_grid, row, text, PaneGrid},
    Application, Command, Element, Settings, Theme,
};
use iced_lazy::responsive;
use root_panes::RootPanesMessage;

pub mod root_panes;
pub mod graph_editor_pane;

#[derive(Debug, Clone)]
pub enum BlackjackUiMessage {
    RootPanes(RootPanesMessage),
}

pub enum BlackjackPane {
    GraphEditor,
    Viewport3d,
    Inspector,
}

struct BlackjackUiApp {
    root_panes: root_panes::RootPanes,
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
        }
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        container(column(vec![
            text("Blackjack").into(),
            self.root_panes.view(),
        ]))
        .into()
    }
}

fn main() {
    BlackjackUiApp::run(Settings::default()).unwrap();
}
