use blackjack_engine::prelude::Itertools;
use iced_native::Renderer;

use crate::prelude::iced_prelude::*;
use crate::prelude::*;

use super::node_widget::NodeWidget;

pub struct NodeEditor<'a> {
    /// The node widgets
    nodes: Vec<BjkUiElement<'a>>,
    /// The offset of each node
    node_positions: Vec<Point>,
}

impl<'a> NodeEditor<'a> {
    pub fn new(nodes: impl Iterator<Item = NodeWidget<'a>>, node_positions: Vec<Point>) -> Self {
        Self {
            nodes: nodes.map(BjkUiElement::new).collect_vec(),
            node_positions,
        }
    }
}

impl<'a> Widget<BjkUiMessage, BjkUiRenderer> for NodeEditor<'a> {
    fn width(&self) -> Length {
        Length::Fill
    }

    fn height(&self) -> Length {
        Length::Fill
    }

    fn diff(&self, tree: &mut iced_native::widget::Tree) {
        tree.diff_children(&self.nodes)
    }

    fn layout(&self, renderer: &BjkUiRenderer, limits: &Limits) -> LayoutNode {
        let mut children = vec![];
        for (ch, pos) in self.nodes.iter().zip(&self.node_positions) {
            // TODO: Limits: Layout as limitless, but perform some kind of culling?
            let layout = ch.as_widget().layout(renderer, limits);
            children.push(layout.translate(pos.to_vector()))
        }
        LayoutNode::with_children(limits.max(), children)
    }

    fn children(&self) -> Vec<WidgetTree> {
        self.nodes.iter().map(WidgetTree::new).collect_vec()
    }

    fn mouse_interaction(
        &self,
        state: &iced_native::widget::Tree,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
        renderer: &BjkUiRenderer,
    ) -> MouseInteraction {
        for ((ch, state), layout) in self
            .nodes
            .iter()
            .zip(state.children.iter())
            .zip(layout.children())
        {
            let interaction = ch.as_widget().mouse_interaction(
                state,
                layout,
                cursor_position,
                viewport,
                renderer,
            );
            if interaction != MouseInteraction::Idle {
                return interaction;
            }
        }
        MouseInteraction::Idle
    }

    fn on_event(
        &mut self,
        state: &mut iced_native::widget::Tree,
        event: iced::Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &BjkUiRenderer,
        clipboard: &mut dyn iced_native::Clipboard,
        shell: &mut iced_native::Shell<'_, BjkUiMessage>,
    ) -> iced::event::Status {
        for ((ch, state), layout) in self
            .nodes
            .iter_mut()
            .zip(state.children.iter_mut())
            .zip(layout.children())
        {
            let status = ch.as_widget_mut().on_event(
                state,
                event.clone(),
                layout,
                cursor_position,
                renderer,
                clipboard,
                shell,
            );
            if status == EventStatus::Captured {
                return status;
            }
        }
        // TODO: Input handling
        EventStatus::Ignored
    }

    fn draw(
        &self,
        state: &iced_native::widget::Tree,
        renderer: &mut BjkUiRenderer,
        theme: &<BjkUiRenderer as iced_native::Renderer>::Theme,
        style: &iced_native::renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) {
        // Draw the background
        renderer.fill_quad(
            Quad {
                bounds: layout.bounds(),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
            Background::Color(theme.background_dark),
        );

        // Draw the nodes
        for ((ch, state), layout) in self
            .nodes
            .iter()
            .zip(state.children.iter())
            .zip(layout.children())
        {
            ch.as_widget().draw(
                state,
                renderer,
                theme,
                style,
                layout,
                cursor_position,
                viewport,
            )
        }
    }
}
