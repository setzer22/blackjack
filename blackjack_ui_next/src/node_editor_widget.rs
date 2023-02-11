use crate::node_widget::NodeWidget;
use epaint::Vec2;
use guee::{
    input::MouseButton,
    painter::TranslateScale,
    prelude::{guee_derives::Builder, *},
};

#[derive(Builder)]
#[builder(widget)]
pub struct NodeEditorWidget {
    pub id: IdGen,
    pub node_widgets: Vec<(Vec2, NodeWidget)>,
    pub pan_zoom: PanZoom,
    #[builder(callback)]
    pub on_pan_zoom_change: Option<Callback<PanZoom>>,
}

#[derive(Copy, Clone, Debug)]
pub struct PanZoom {
    pub pan: Vec2,
    pub zoom: f32,
}

impl Default for PanZoom {
    fn default() -> Self {
        Self { pan: Vec2::ZERO, zoom: 1.0 }
    }
}

impl PanZoom {
    /// Increments the current zoom by zoom_level, and increases the current
    /// zoom level by `zoom_delta` and adjusts the panning so that zoom is
    /// centered around the given `point`.
    ///
    /// The point is provided in window-space coordinates, relative to the
    /// top-left corner of the graph.
    pub fn adjust_zoom(&mut self, zoom_delta: f32, point: Pos2, zoom_min: f32, zoom_max: f32) {
        // Adjust the zoom level, taking min / max into account.
        let zoom_new = {
            let clamped = (self.zoom + zoom_delta).clamp(zoom_min, zoom_max);
            let delta_clamped = clamped - self.zoom;
            self.zoom * (1.0 + delta_clamped)
        };

        // To adjust the pan, we consider the point at the previous zoom level,
        // and the position where that point ends up after modifying the zoom
        // level if we didn't correct the pan. We then shift the view in the
        // opposite direction to keep that point at the same position.
        //
        // NOTE: The points at current and new zoom level are obtained by
        // dividing the cursor position by the zoom. Division is done to apply
        // the inverse transformation, since we are converting from screen space
        // to graph space, not vice-versa. We ignore pan in the transformation
        // because we're only interested in the difference.
        let point = point.to_vec2();
        let pan_correction = point / zoom_new - point / self.zoom;

        self.pan += pan_correction;
        self.zoom = zoom_new;
        dbg!(self);
    }
}

impl NodeEditorWidget {
    // Given the screen coordinates of the top-left corner of the node editor,
    // returns the the direct transform to be applied to the nodes when
    // rendering them.
    pub fn direct_transform(&self, top_left: Vec2) -> TranslateScale {
        TranslateScale::identity()
            .translated(-top_left)
            .translated(self.pan_zoom.pan)
            .scaled(self.pan_zoom.zoom)
            .translated(top_left)
    }

    // Given the screen coordinates of the top-left corner of the node editor,
    // returns the the cursor transform that needs to be applied to convert the
    // cursor position in screen coordiantes to the cursor position inside the
    // node editor.
    pub fn cursor_transform(&self, top_left: Vec2) -> TranslateScale {
        TranslateScale::identity()
            .translated(-top_left)
            .scaled(1.0 / self.pan_zoom.zoom)
            .translated(-self.pan_zoom.pan)
            .translated(top_left)
    }
}

impl Widget for NodeEditorWidget {
    fn layout(&mut self, ctx: &Context, parent_id: WidgetId, available: Vec2) -> Layout {
        // Strategy: Layout normally, then draw and handle events with panned / scaled
        let widget_id = self.id.resolve(parent_id);
        let mut children = vec![];
        for (pos, nw) in &mut self.node_widgets {
            children.push(nw.layout(ctx, widget_id, available).translated(*pos))
        }
        Layout::with_children(widget_id, available, children)
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        let old_transform = ctx.painter().transform;
        let top_left = layout.bounds.left_top();
        ctx.painter().transform = self.direct_transform(top_left.to_vec2());
        // TODO: Set clip rect
        for ((_pos, node_widget), node_layout) in self.node_widgets.iter_mut().zip(&layout.children)
        {
            node_widget.draw(ctx, node_layout)
        }
        ctx.painter().transform = old_transform;
    }

    fn min_size(&mut self, _ctx: &Context, available: Vec2) -> Vec2 {
        // Gimme all you got
        available
    }

    fn layout_hints(&self) -> LayoutHints {
        LayoutHints::fill()
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        events: &[Event],
    ) -> EventStatus {
        let top_left = layout.bounds.left_top();
        let transformed_cursor_position = self
            .cursor_transform(top_left.to_vec2())
            .transform_point(cursor_position);
        for ((_pos, node_widget), node_layout) in self.node_widgets.iter_mut().zip(&layout.children)
        {
            if let EventStatus::Consumed =
                node_widget.on_event(ctx, node_layout, transformed_cursor_position, events)
            {
                return EventStatus::Consumed;
            }
        }

        let mut event_status = EventStatus::Ignored;
        let contains_cursor = layout.bounds.contains(cursor_position);
        for event in events {
            match event {
                Event::MousePressed(MouseButton::Middle) if contains_cursor => {}
                Event::MouseReleased(MouseButton::Middle) => {}
                Event::MouseMoved(_) => {
                    // The mouse delta, in untransformed screen-space units
                    let delta_screen = ctx.input_state.mouse_state.delta();
                }
                Event::MouseWheel(scroll) if contains_cursor => {
                    // WIP:
                    // - Zoom quickly goes to "inf", it's broken
                    // - The painter does not take the current transform into account
                    println!("Mouse wheel {scroll:?}");
                    self.pan_zoom.adjust_zoom(
                        scroll.y * 0.05,
                        cursor_position - layout.bounds.left_top().to_vec2(),
                        0.25,
                        3.0,
                    );
                    if let Some(cb) = self.on_pan_zoom_change.take() {
                        ctx.dispatch_callback(cb, self.pan_zoom);
                    }
                    event_status = EventStatus::Consumed;
                }
                _ => {}
            }
        }

        event_status
    }
}
