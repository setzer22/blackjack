use blackjack_engine::graph::BjkGraph;
use iced::{
    executor,
    widget::{button, column, container, pane_grid, row, text, PaneGrid},
    Application, Command, Element, Settings, Theme,
};
use iced_lazy::responsive;

use crate::BlackjackUiMessage;

use super::graph_editor_pane::GraphEditorPane;

#[derive(Copy, Clone, Debug)]
pub enum RootPanesMessage {
    PaneResized(pane_grid::ResizeEvent),
    PaneClicked(pane_grid::Pane),
    PaneDragged(pane_grid::DragEvent),
}

pub enum BlackjackPane {
    GraphEditor(GraphEditorPane),
    Viewport3d,
    Inspector,
    Spreadsheet,
}

impl Into<BlackjackUiMessage> for RootPanesMessage {
    fn into(self) -> BlackjackUiMessage {
        BlackjackUiMessage::RootPanes(self)
    }
}

/// The container for the panes, allowing horizontal / vertical divisions
pub struct RootPanes {
    panes: pane_grid::State<BlackjackPane>,
}

impl RootPanes {
    pub fn new() -> Self {
        let (mut panes, viewport) = pane_grid::State::new(BlackjackPane::Viewport3d);
        panes.split(
            pane_grid::Axis::Horizontal,
            &viewport,
            BlackjackPane::GraphEditor(GraphEditorPane::default()),
        );
        let (_, split) = panes
            .split(
                pane_grid::Axis::Vertical,
                &viewport,
                BlackjackPane::Inspector,
            )
            .unwrap();
        panes.resize(&split, 0.6);
        Self { panes }
    }

    pub fn update(&mut self, message: RootPanesMessage) {
        match message {
            RootPanesMessage::PaneResized(resize) => self.panes.resize(&resize.split, resize.ratio),
            RootPanesMessage::PaneClicked(_) => {}
            RootPanesMessage::PaneDragged(dragged) => match dragged {
                pane_grid::DragEvent::Dropped { pane, target } => {
                    self.panes.swap(&pane, &target);
                }
                pane_grid::DragEvent::Picked { .. } => {}
                pane_grid::DragEvent::Canceled { .. } => {}
            },
        }
    }

    pub fn view<'a>(&'a self, graph: &'a BjkGraph) -> iced::Element<'_, super::BlackjackUiMessage> {
        let pane_grid = PaneGrid::new(&self.panes, |id, pane, maximized| {
            let title = row![match pane {
                BlackjackPane::GraphEditor(g) => g.titlebar_view(graph),
                BlackjackPane::Viewport3d => text("Viewport 3d").into(),
                BlackjackPane::Inspector => text("Inspector").into(),
                BlackjackPane::Spreadsheet => text("Spreadsheet").into(),
            }];

            let title_bar = pane_grid::TitleBar::new(title);

            pane_grid::Content::new(responsive(move |size| {
                match pane {
                    BlackjackPane::GraphEditor(g) => g.content_view(graph),
                    BlackjackPane::Viewport3d => text("I am the 3d viewport").into(),
                    BlackjackPane::Inspector => text("I am the inspector 🕵").into(),
                    BlackjackPane::Spreadsheet => text("I am the mighty spreadsheet").into(),
                }
                .into()
            }))
            .title_bar(title_bar)
            .style(style::pane_active as for<'r> fn(&'r _) -> _)
        })
        .style(iced::theme::PaneGrid::Custom(Box::new(
            style::PaneGridStyle,
        )))
        .spacing(5)
        .on_click(|x| RootPanesMessage::PaneClicked(x).into())
        .on_drag(|x| RootPanesMessage::PaneDragged(x).into())
        .on_resize(5, |x| RootPanesMessage::PaneResized(x).into());

        pane_grid.into()
    }
}

mod style {
    use iced::{
        widget::{container, pane_grid::Line},
        Color, Theme,
    };

    pub fn pane_active(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            background: Some(palette.background.weak.color.into()),
            border_width: 2.0,
            border_color: palette.background.strong.color,
            ..Default::default()
        }
    }

    #[derive(Default)]
    pub struct PaneGridStyle;

    impl iced::widget::pane_grid::StyleSheet for PaneGridStyle {
        type Style = Theme;

        fn picked_split(&self, style: &Self::Style) -> Option<Line> {
            Some(Line {
                color: Color::from_rgb(1.0, 0.0, 0.0),
                width: 3.0,
            })
        }

        fn hovered_split(&self, style: &Self::Style) -> Option<Line> {
            Some(Line {
                color: Color::from_rgb(0.0, 1.0, 0.0),
                width: 3.0,
            })
        }
    }
}
