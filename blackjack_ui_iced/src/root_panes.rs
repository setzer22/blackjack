use blackjack_engine::graph::BjkGraph;
use iced::widget::pane_grid;
use iced_lazy::responsive;

use crate::BjkUiMessage;

use super::graph_editor_pane::GraphEditorPane;

use crate::prelude::*;

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

impl From<RootPanesMessage> for BjkUiMessage {
    fn from(val: RootPanesMessage) -> Self {
        BjkUiMessage::RootPanes(val)
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

    pub fn view<'a>(&'a self, graph: &'a BjkGraph) -> BjkUiElement<'a> {
        let pane_grid = pane_grid::PaneGrid::new(&self.panes, |_id, pane, _maximized| {
            let title = match pane {
                BlackjackPane::GraphEditor(g) => g.titlebar_view(graph),
                BlackjackPane::Viewport3d => text("Viewport 3d").into(),
                BlackjackPane::Inspector => text("Inspector").into(),
                BlackjackPane::Spreadsheet => text("Spreadsheet").into(),
            };

            let title_bar = pane_grid::TitleBar::new(title);

            pane_grid::Content::new(responsive(move |_size| match pane {
                BlackjackPane::GraphEditor(g) => g.content_view(graph),
                BlackjackPane::Viewport3d => text("I am the 3d viewport").into(),
                BlackjackPane::Inspector => text("I am the inspector 🕵").into(),
                BlackjackPane::Spreadsheet => text("I am the mighty spreadsheet").into(),
            }))
            .title_bar(title_bar)
            .style(BjkContainerStyle::Pane)
        })
        .spacing(5)
        .on_click(|x| RootPanesMessage::PaneClicked(x).into())
        .on_drag(|x| RootPanesMessage::PaneDragged(x).into())
        .on_resize(5, |x| RootPanesMessage::PaneResized(x).into());

        pane_grid.into()
    }
}

impl Default for RootPanes {
    fn default() -> Self {
        Self::new()
    }
}
