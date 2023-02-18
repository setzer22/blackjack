use std::any::type_name;

use blackjack_engine::{
    graph::{BjkNodeId, DataType},
    graph_interpreter::BjkParameter,
};
use epaint::{CircleShape, RectShape, Rounding};
use guee::{input::MouseButton, prelude::*};
use itertools::Itertools;

pub struct NodeWidgetPort {
    pub color: Color32,
    pub data_type: DataType,
    pub hovered: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PortIdKind {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortId {
    pub param: BjkParameter,
    pub side: PortIdKind,
    pub data_type: DataType,
}

impl PortId {
    pub fn is_compatible(&self, other: &PortId) -> bool {
        self.side != other.side && self.data_type == other.data_type
    }
}

pub struct NodeWidgetRow {
    pub input_port: Option<NodeWidgetPort>,
    pub contents: DynWidget,
    pub output_port: Option<NodeWidgetPort>,
}

pub struct NodeWidget {
    pub id: IdGen,
    pub node_id: BjkNodeId,
    pub titlebar_left: DynWidget,
    pub titlebar_right: DynWidget,
    pub bottom_ui: DynWidget,
    pub rows: Vec<(BjkParameter, NodeWidgetRow)>,

    pub v_separation: f32,
    pub h_separation: f32,
    pub extra_v_separation: f32,

    pub on_node_dragged: Option<Callback<Vec2>>,
}

pub struct NodeWidgetState {
    pub dragging: bool,
}

impl NodeWidget {
    pub const PORT_RADIUS: f32 = 5.0;

    /// Returns the bounding box of the titlebar, given the `layout` tree.
    fn titlebar_rect(&self, layout: &Layout) -> Rect {
        let node = layout.bounds;
        let tb_left = &layout.children[0].bounds;
        let tb_right = &layout.children[1].bounds;

        Rect::from_min_size(
            Pos2::new(tb_left.left() - self.h_separation, tb_left.top()),
            Vec2::new(
                node.width(),
                tb_left.height().max(tb_right.height()) + self.extra_v_separation,
            ),
        )
    }

    /// Returns the visual information of the left and right port (both
    /// optional) for the `row-idx`-th row
    #[allow(clippy::type_complexity)]
    pub fn port_visuals(
        &self,
        layout: &Layout,
        param: &BjkParameter,
    ) -> (Option<(Pos2, Color32)>, Option<(Pos2, Color32)>) {
        let (row_idx, (_, row)) = self
            .rows
            .iter()
            .find_position(|(p, _row)| p == param)
            .expect("Invalid param");
        let row_bounds = layout.children[row_idx + 2].bounds;
        let node_bounds = layout.bounds;
        let left = Pos2::new(node_bounds.left(), row_bounds.center().y);
        let right = Pos2::new(
            node_bounds.left() + node_bounds.width(),
            row_bounds.center().y,
        );
        (
            row.input_port
                .as_ref()
                .map(|i| (left, if i.hovered { Color32::WHITE } else { i.color })),
            row.output_port
                .as_ref()
                .map(|o| (right, if o.hovered { Color32::WHITE } else { o.color })),
        )
    }
}

impl Widget for NodeWidget {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        if force_shrink {
            SizeHint::ignore_force_warning(type_name::<Self>())
        }

        let widget_id = self.id.resolve(parent_id);

        struct Cursor {
            y_offset: f32,
            available: Vec2,
            total_size: Vec2,
        }

        let mut cursor = Cursor {
            y_offset: self.v_separation,
            available,
            total_size: Vec2::new(0.0, 0.0),
        };

        let layout_widget = |w: &mut DynWidget, c: &mut Cursor| -> Layout {
            let layout = w.widget.layout(ctx, widget_id, c.available, false);
            let size = layout.bounds.size();
            c.available -= Vec2::new(0.0, size.y + self.v_separation);
            c.total_size.x = c.total_size.x.max(size.x);
            c.total_size.y += size.y + self.v_separation;
            let layout = layout.translated(Vec2::new(self.h_separation, c.y_offset));
            c.y_offset += size.y + self.v_separation;
            layout
        };

        let title_left_layout = self
            .titlebar_left
            .widget
            .layout(ctx, widget_id, available, false);
        let title_right_layout = self
            .titlebar_right
            .widget
            .layout(ctx, widget_id, available, false);
        let title_height = title_left_layout
            .bounds
            .size()
            .y
            .max(title_right_layout.bounds.size().y)
            + self.extra_v_separation;
        cursor.y_offset += title_height;
        cursor.total_size.x = cursor
            .total_size
            .x
            .max(title_left_layout.bounds.size().x + title_right_layout.bounds.size().x);
        cursor.total_size.y += title_height;

        // Layout row contents
        let mut row_contents = vec![];
        for (_, row) in &mut self.rows {
            let row_layout = layout_widget(&mut row.contents, &mut cursor);
            row_contents.push(row_layout);
        }

        // Layout bottom UI
        cursor.y_offset += self.v_separation;
        let bottom_ui_layout = layout_widget(&mut self.bottom_ui, &mut cursor);

        // Layout titlebar
        let trl_width = title_right_layout.bounds.width();
        let title_left_layout = title_left_layout.translated(Vec2::new(self.h_separation, 0.0));
        let title_right_layout = title_right_layout.translated(Vec2::new(
            cursor.total_size.x - trl_width + self.h_separation,
            0.0,
        ));

        cursor.total_size.y += 3.0 * self.v_separation;
        cursor.total_size.x += 2.0 * self.h_separation;

        let mut children = vec![];
        children.push(title_left_layout);
        children.push(title_right_layout);
        for row in row_contents {
            children.push(row);
        }
        children.push(bottom_ui_layout);

        Layout::with_children(widget_id, cursor.total_size, children)
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        let border_radius = 5.0;

        let node_rect = layout.bounds;

        // Draw the node background
        ctx.painter().rect(RectShape {
            rect: node_rect,
            rounding: Rounding::same(border_radius),
            fill: color!("#3f3f3f"), // TODO Pallette?
            stroke: Stroke::NONE,
        });

        // Draw the titlebar on top
        let title_rect = self.titlebar_rect(layout);
        ctx.painter().rect(RectShape {
            rect: title_rect,
            rounding: Rounding {
                nw: border_radius,
                ne: border_radius,
                sw: 0.0,
                se: 0.0,
            },
            fill: color!("#323232"),
            stroke: Stroke::NONE,
        });

        // Draw all the child widgets
        self.titlebar_left.widget.draw(ctx, &layout.children[0]);
        self.titlebar_right.widget.draw(ctx, &layout.children[1]);
        let row_wgt_layouts = &layout.children[2..2 + self.rows.len()];
        for ((_param, row), row_layout) in self.rows.iter_mut().zip(row_wgt_layouts) {
            row.contents.widget.draw(ctx, row_layout);
        }
        self.bottom_ui
            .widget
            .draw(ctx, &layout.children[2 + self.rows.len()]);

        for (param, _row) in self.rows.iter() {
            let (left, right) = self.port_visuals(layout, param);
            if let Some((left, color)) = left {
                ctx.painter().circle(CircleShape {
                    center: left,
                    radius: Self::PORT_RADIUS,
                    fill: color,
                    stroke: Stroke::NONE,
                })
            }
            if let Some((right, color)) = right {
                ctx.painter().circle(CircleShape {
                    center: right,
                    radius: Self::PORT_RADIUS,
                    fill: color,
                    stroke: Stroke::NONE,
                })
            }
        }
    }

    fn layout_hints(&self) -> LayoutHints {
        unimplemented!(
            "A node widget should not be inside a container that checks its layout hints"
        )
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        events: &[Event],
    ) -> EventStatus {
        if let EventStatus::Consumed =
            self.titlebar_left
                .widget
                .on_event(ctx, &layout.children[0], cursor_position, events)
        {
            return EventStatus::Consumed;
        }
        if let EventStatus::Consumed =
            self.titlebar_right
                .widget
                .on_event(ctx, &layout.children[1], cursor_position, events)
        {
            return EventStatus::Consumed;
        }
        let row_layouts = &layout.children[2..2+self.rows.len()];
        for ((_, row), row_layout) in self.rows.iter_mut().zip(row_layouts) {
            if let EventStatus::Consumed =
                row.contents
                    .widget
                    .on_event(ctx, row_layout, cursor_position, events)
            {
                return EventStatus::Consumed;
            }
        }
        if let EventStatus::Consumed = self.bottom_ui.widget.on_event(
            ctx,
            &layout.children[2 + self.rows.len()],
            cursor_position,
            events,
        ) {
            return EventStatus::Consumed;
        }

        let titlebar_rect = self.titlebar_rect(layout);
        let mut state = ctx
            .memory
            .get_mut_or(layout.widget_id, NodeWidgetState { dragging: false });
        let is_in_titlebar = titlebar_rect.contains(cursor_position);

        let mut status = EventStatus::Ignored;
        for event in events {
            match event {
                Event::MousePressed(MouseButton::Primary) if is_in_titlebar => {
                    state.dragging = true;
                    status = EventStatus::Consumed;
                }
                Event::MouseReleased(MouseButton::Primary) => {
                    state.dragging = false;
                    return EventStatus::Ignored;
                }
                _ => {}
            }
        }

        if state.dragging {
            let delta = ctx.input_state.mouse_state.delta();
            if let Some(cb) = self.on_node_dragged.take() {
                ctx.dispatch_callback(cb, delta);
            }
        }

        status
    }
}
