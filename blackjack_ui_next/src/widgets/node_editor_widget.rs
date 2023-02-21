use std::any::type_name;

use blackjack_engine::{graph::BjkNodeId, graph_interpreter::BjkParameter};
use epaint::{CircleShape, CubicBezierShape, RectShape, Rounding, Vec2};
use guee::{
    extension_traits::{Color32Ext, Vec2Ext},
    input::MouseButton,
    painter::TranslateScale,
    prelude::{guee_derives::Builder, *},
};
use itertools::Itertools;

use crate::{pallette, widgets::node_widget::PortIdKind};

use super::node_widget::{NodeWidget, PortId};

pub struct Connection {
    pub input: BjkParameter,
    pub output: BjkParameter,
}

pub struct Disconnection {
    pub input: BjkParameter,
    pub output: BjkParameter,
}

#[derive(Builder)]
#[builder(widget, skip_new)]
pub struct NodeEditorWidget {
    pub id: IdGen,
    pub node_widgets: Vec<(Vec2, NodeWidget)>,
    pub connections: Vec<(PortId, PortId)>,
    pub pan_zoom: PanZoom,
    #[builder(strip_option)]
    pub on_pan_zoom_change: Option<Callback<PanZoom>>,
    /// Callback is guaranteed to get passed an input and output ports (not two
    /// inputs, or two outputs), order isn't guaranteed.
    #[builder(strip_option)]
    pub on_connection: Option<Callback<Connection>>,
    /// Callback is guaranteed to get passed an input and output ports (not two
    /// inputs, or two outputs), order isn't guaranteed.
    #[builder(strip_option)]
    pub on_disconnection: Option<Callback<Disconnection>>,
    /// Will get called when a node is interacted with in a way that would
    /// require raising it to the top of the node order.
    #[builder(strip_option)]
    pub on_node_raised: Option<Callback<BjkNodeId>>,
}

pub struct NodeEditorWidgetState {
    pub ongoing_connection: Option<PortId>,
}

#[derive(Copy, Clone, Debug)]
pub struct PanZoom {
    pub pan: Vec2,
    pub zoom: f32,
}

impl Default for PanZoom {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
        }
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
    }
}

impl NodeEditorWidget {
    pub fn new(
        id_gen: IdGen,
        node_widgets: Vec<(Vec2, NodeWidget)>,
        connections: Vec<(PortId, PortId)>,
        pan_zoom: PanZoom,
    ) -> Self {
        Self {
            id: id_gen,
            node_widgets,
            connections,
            pan_zoom,
            on_pan_zoom_change: None,
            on_connection: None,
            on_disconnection: None,
            on_node_raised: None,
        }
    }

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

    /// Returns the currently hovered port, if any. Also marks the port itself
    /// as hovered (by mutating it) so that it can react to it when being drawn
    /// during the draw phase.
    pub fn find_hovered_port(&mut self, cursor_position: Pos2, layout: &Layout) -> Option<PortId> {
        for ((_, node), node_layout) in self.node_widgets.iter_mut().zip(&layout.children) {
            let mut hovered_row = None;
            for (row_idx, (param, row)) in node.rows.iter().enumerate() {
                let (left, right) = node.port_visuals(node_layout, param);

                macro_rules! find_port {
                    ($accessor:ident) => {
                        row.$accessor.as_ref().expect("Port should be input")
                    };
                }

                if let Some((left_pos, _)) = left {
                    if cursor_position.distance(left_pos) < NodeWidget::PORT_RADIUS {
                        let port = find_port!(input_port);
                        hovered_row = Some((
                            row_idx,
                            PortIdKind::Input,
                            PortId {
                                param: param.clone(),
                                side: PortIdKind::Input,
                                data_type: port.data_type,
                            },
                        ));
                        break;
                    }
                }
                if let Some((right_pos, _)) = right {
                    if cursor_position.distance(right_pos) < NodeWidget::PORT_RADIUS {
                        let port = find_port!(output_port);
                        hovered_row = Some((
                            row_idx,
                            PortIdKind::Output,
                            PortId {
                                param: param.clone(),
                                side: PortIdKind::Output,
                                data_type: port.data_type,
                            },
                        ));
                        break;
                    }
                }
            }
            if let Some((row_idx, side, port_id)) = hovered_row {
                let row = &mut node.rows[row_idx].1;
                match side {
                    PortIdKind::Input => row.input_port.as_mut().unwrap().hovered = true,
                    PortIdKind::Output => row.output_port.as_mut().unwrap().hovered = true,
                }
                return Some(port_id);
            }
        }
        None
    }

    pub fn port_pos(&self, layout: &Layout, port_id: &PortId) -> Pos2 {
        let (node_idx, (_, node_widget)) = self
            .node_widgets
            .iter()
            .find_position(|(_, n)| n.node_id == port_id.param.node_id)
            .unwrap();
        let node_layout = &layout.children[node_idx];
        let (left, right) = node_widget.port_visuals(node_layout, &port_id.param);

        // NOTE: This assumes each row has either an input, or an output
        match port_id.side {
            PortIdKind::Input => left.unwrap().0,
            PortIdKind::Output => right.unwrap().0,
        }
    }

    pub fn connection_shape(&self, src: Pos2, dst: Pos2) -> CubicBezierShape {
        let stroke = Stroke::new(5.0, pallette().widget_fg);
        let control_scale = ((dst.x - src.x) / 2.0).max(30.0);
        let src_control = src + Vec2::X * control_scale;
        let dst_control = dst - Vec2::X * control_scale;
        CubicBezierShape {
            points: [src, src_control, dst_control, dst],
            closed: false,
            fill: Color32::TRANSPARENT,
            stroke,
        }
    }
}

impl Widget for NodeEditorWidget {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool, // ignored, not expanded.
    ) -> Layout {
        if force_shrink {
            SizeHint::ignore_force_warning(type_name::<Self>());
        }

        // Strategy: Layout normally, then draw and handle events with panned / scaled
        let widget_id = self.id.resolve(parent_id);
        let mut children = vec![];
        for (pos, nw) in &mut self.node_widgets {
            children.push(nw.layout(ctx, widget_id, available, false).translated(*pos))
        }
        Layout::with_children(widget_id, available, children)
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        let top_left = layout.bounds.left_top();

        // Set clip rect
        let old_clip_rect = ctx.painter().clip_rect;
        ctx.painter().clip_rect = layout.bounds;

        // Draw background
        {
            let scale = self.pan_zoom.zoom;

            const GRID_SIZE: f32 = 30.0;
            let x_size = (layout.bounds.width() / GRID_SIZE / scale) as i32;
            let y_size = (layout.bounds.height() / GRID_SIZE / scale) as i32;
            let mut painter = ctx.painter();

            painter.rect(RectShape {
                rect: layout.bounds,
                rounding: Rounding::none(),
                fill: pallette().background_dark,
                stroke: Stroke::NONE,
            });

            let radius = 1.5 * scale;

            let offset = self
                .pan_zoom
                .pan
                .rem_euclid(Vec2::new(GRID_SIZE, GRID_SIZE));

            for y in -1..y_size + 1 {
                for x in -1..x_size + 1 {
                    let center = layout.bounds.left_top()
                        + Vec2::new(x as f32 * GRID_SIZE * scale, y as f32 * GRID_SIZE * scale)
                        + Vec2::new(1.0, 1.0) * radius * 2.0;
                    let center = (center.to_vec2() + (offset * scale)).to_pos2();

                    painter.circle(CircleShape {
                        center,
                        radius,
                        fill: pallette().widget_fg_dark.with_alpha(20),
                        stroke: Stroke::NONE,
                    })
                }
            }
        }

        // Setup transformation
        let old_transform = ctx.painter().transform;
        ctx.painter().transform = self.direct_transform(top_left.to_vec2());

        // Draw existing connections
        for (src, dst) in &self.connections {
            let src_pos = self.port_pos(layout, dst);
            let dst_pos = self.port_pos(layout, src);
            ctx.painter()
                .cubic_bezier(self.connection_shape(src_pos, dst_pos));
        }

        // Draw ongoing connection
        let state = ctx.memory.get::<NodeEditorWidgetState>(layout.widget_id);
        if let Some(ongoing) = &state.ongoing_connection {
            let port_pos = self.port_pos(layout, ongoing);
            let mouse_pos = self
                .cursor_transform(layout.bounds.left_top().to_vec2())
                .transform_point(ctx.input_state.mouse.position);
            ctx.painter()
                .cubic_bezier(self.connection_shape(port_pos, mouse_pos));
        }
        drop(state);

        // Draw nodes
        for ((_pos, node_widget), node_layout) in self.node_widgets.iter_mut().zip(&layout.children)
        {
            node_widget.draw(ctx, node_layout)
        }

        // Restore transformation & clip rect
        ctx.painter().clip_rect = old_clip_rect;
        ctx.painter().transform = old_transform;
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
        let cursor_transform = self.cursor_transform(top_left.to_vec2());
        let transformed_cursor_position = cursor_transform.transform_point(cursor_position);

        // Set the cursor transform state. This is necessary for child widgets
        // to be able to properly track click / drag events.
        let prev_tr = ctx.input_widget_state.borrow().cursor_transform;
        ctx.input_widget_state.borrow_mut().cursor_transform = cursor_transform;

        let mut event_status = EventStatus::Ignored;

        // NOTE: This needs to be iterated in reverse, so nodes are drawn
        // bottom-to-top, but events processed top-to-bottom.
        for ((_pos, node_widget), node_layout) in
            self.node_widgets.iter_mut().zip(&layout.children).rev()
        {
            if let EventStatus::Consumed =
                node_widget.on_event(ctx, node_layout, transformed_cursor_position, events)
            {
                if let Some(on_raised) = self.on_node_raised.take() {
                    ctx.dispatch_callback(on_raised, node_widget.node_id);
                }

                event_status = EventStatus::Consumed;
                break;
            }
        }

        // Restore the previous cursor transform.
        ctx.input_widget_state.borrow_mut().cursor_transform = prev_tr;

        // We do this here, and not directly inside the loop, to avoid ending
        // the frame with a modified cursor transform.
        if event_status == EventStatus::Consumed {
            return event_status;
        }

        let mut state = ctx.memory.get_mut_or(
            layout.widget_id,
            NodeEditorWidgetState {
                ongoing_connection: None,
            },
        );

        let contains_cursor = layout.bounds.contains(cursor_position);

        // Check events on ports
        let primary_clicked = events
            .iter()
            .any(|ev| matches!(&ev, Event::MousePressed(MouseButton::Primary)));
        let primary_released = events
            .iter()
            .any(|ev| matches!(&ev, Event::MouseReleased(MouseButton::Primary)));

        let prev_ongoing = state.ongoing_connection.clone();
        if let Some(hovered) = self.find_hovered_port(transformed_cursor_position, layout) {
            match prev_ongoing {
                Some(ongoing) => {
                    if primary_released && hovered.is_compatible(&ongoing) {
                        if let Some(cb) = self.on_connection.take() {
                            let (input, output) = if hovered.side == PortIdKind::Input {
                                (hovered, ongoing)
                            } else {
                                (ongoing, hovered)
                            };
                            ctx.dispatch_callback(
                                cb,
                                Connection {
                                    input: input.param,
                                    output: output.param,
                                },
                            );
                        }
                        state.ongoing_connection = None;
                        return EventStatus::Consumed;
                    }
                }
                None => {
                    if primary_clicked {
                        let already_connected_to = self.connections.iter().find_map(|(a, b)| {
                            if a == &hovered {
                                Some(b)
                            } else if b == &hovered {
                                Some(a)
                            } else {
                                None
                            }
                        });
                        if let Some(already) = already_connected_to {
                            if let Some(cb) = self.on_disconnection.take() {
                                let (input, output) = if hovered.side == PortIdKind::Input {
                                    (hovered, already.clone())
                                } else {
                                    (already.clone(), hovered)
                                };
                                ctx.dispatch_callback(
                                    cb,
                                    Disconnection {
                                        input: input.param,
                                        output: output.param.clone(),
                                    },
                                );
                                state.ongoing_connection = Some(output);
                            }
                        } else {
                            state.ongoing_connection = Some(hovered);
                            return EventStatus::Consumed;
                        }
                    }
                }
            }
        }
        if primary_released {
            // If this was a connection end, we would've returned by now.
            state.ongoing_connection = None;
        }

        for event in events {
            match event {
                Event::MouseWheel(scroll) if contains_cursor => {
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

        let panning = ctx.claim_drag_event(layout.widget_id, layout.bounds, MouseButton::Middle);
        if panning {
            let delta_screen = ctx.input_state.mouse.delta() / self.pan_zoom.zoom;
            self.pan_zoom.pan += delta_screen;
            if let Some(cb) = self.on_pan_zoom_change.take() {
                ctx.dispatch_callback(cb, self.pan_zoom);
            }
            event_status = EventStatus::Consumed;
        }

        event_status
    }
}
