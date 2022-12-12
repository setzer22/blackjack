use crate::iced_prelude::*;
use crate::prelude::*;
use iced_native::overlay;
use iced_native::Renderer;

/// An element to display a widget over another.
pub struct OverlapContainer<'a> {
    pub bg_widget: BjkUiElement<'a>,
    pub fg_widget: BjkUiElement<'a>,
    pub fg_widget_position: Vector,
}

impl<'a> OverlapContainer<'a> {
    pub fn new(
        content: impl Into<BjkUiElement<'a>>,
        overlay: impl Into<BjkUiElement<'a>>,
        overlay_position: Vector,
    ) -> Self {
        Self {
            bg_widget: content.into(),
            fg_widget: overlay.into(),
            fg_widget_position: overlay_position,
        }
    }
}

impl<'a> Widget<BjkUiMessage, BjkUiRenderer> for OverlapContainer<'a> {
    fn children(&self) -> Vec<WidgetTree> {
        vec![
            WidgetTree::new(&self.bg_widget),
            WidgetTree::new(&self.fg_widget),
        ]
    }

    fn diff(&self, tree: &mut WidgetTree) {
        tree.diff_children(&[&self.bg_widget, &self.fg_widget])
    }

    fn width(&self) -> Length {
        self.bg_widget.as_widget().width()
    }

    fn height(&self) -> Length {
        self.bg_widget.as_widget().height()
    }

    fn layout(&self, renderer: &BjkUiRenderer, limits: &Limits) -> LayoutNode {
        let bg_layout = self.bg_widget.as_widget().layout(renderer, limits);
        let fg_layout = self
            .fg_widget
            .as_widget()
            .layout(renderer, limits)
            .translate(self.fg_widget_position);

        LayoutNode::with_children(bg_layout.size(), vec![bg_layout, fg_layout])
    }

    fn on_event(
        &mut self,
        tree: &mut WidgetTree,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &BjkUiRenderer,
        clipboard: &mut dyn iced_native::Clipboard,
        shell: &mut iced_native::Shell<'_, BjkUiMessage>,
    ) -> iced::event::Status {
        if self.fg_widget.as_widget_mut().on_event(
            &mut tree.children[1],
            event.clone(),
            layout.children().nth(1).unwrap(),
            cursor_position,
            renderer,
            clipboard,
            shell,
        ) == EventStatus::Captured
        {
            return EventStatus::Captured;
        };

        self.bg_widget.as_widget_mut().on_event(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            cursor_position,
            renderer,
            clipboard,
            shell,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &WidgetTree,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
        renderer: &BjkUiRenderer,
    ) -> MouseInteraction {
        let fg_interaction = self.fg_widget.as_widget().mouse_interaction(
            &tree.children[1],
            layout.children().nth(1).unwrap(),
            cursor_position,
            viewport,
            renderer,
        );

        if fg_interaction != MouseInteraction::Idle {
            return fg_interaction;
        }

        self.bg_widget.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor_position,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &WidgetTree,
        renderer: &mut BjkUiRenderer,
        theme: &<BjkUiRenderer as Renderer>::Theme,
        inherited_style: &iced_native::renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) {
        renderer.with_layer(layout.bounds(), |renderer| {
            self.bg_widget.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                inherited_style,
                layout.children().next().unwrap(),
                cursor_position,
                viewport,
            );
        });
        renderer.with_layer(layout.bounds(), |renderer| {
            self.fg_widget.as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                inherited_style,
                layout.children().nth(1).unwrap(),
                cursor_position,
                viewport,
            );
        });
    }

    fn overlay<'b>(
        &'b self,
        tree: &'b mut WidgetTree,
        layout: Layout<'_>,
        renderer: &BjkUiRenderer,
    ) -> Option<overlay::Element<'b, BjkUiMessage, BjkUiRenderer>> {
        self.bg_widget
            .as_widget()
            .overlay(&mut tree.children[0], layout, renderer)
    }
}
