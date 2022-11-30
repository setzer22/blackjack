use blackjack_engine::graph::BjkGraph;
use iced::widget::pane_grid;
use iced_lazy::{responsive, Responsive};

use crate::{graph_editor_pane::GraphEditorState, BjkUiMessage, BlackjackUiApp};

use super::graph_editor_pane::GraphEditorPane;

use crate::prelude::*;

#[derive(Copy, Clone, Debug)]
pub enum RootPanesMessage {
    PaneResized(pane_grid::ResizeEvent),
    PaneClicked(pane_grid::Pane),
    PaneDragged(pane_grid::DragEvent),
}

#[derive(Clone, Copy)]
pub enum BlackjackPane {
    GraphEditor,
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
            BlackjackPane::GraphEditor,
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

    pub fn view<'a>(
        &'a self,
        app_state: &'a BlackjackUiApp,
        pane_title: fn(&'a BlackjackUiApp, BlackjackPane) -> BjkUiElement<'a>,
        pane_view: fn(&'a BlackjackUiApp, BlackjackPane) -> BjkUiElement<'a>,
    ) -> BjkUiElement<'a> {
        let pane_grid = pane_grid::PaneGrid::new(&self.panes, |_id, pane, _maximized| {
            let title = pane_title(app_state, *pane);

            let title_bar = pane_grid::TitleBar::new(title);

            let pane = *pane;
            pane_grid::Content::new(Responsive::new(move |_| pane_view(app_state, pane)))
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
