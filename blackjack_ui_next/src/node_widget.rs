use guee::prelude::*;

pub struct NodeWidget {}

// WIP: We need a separate NodeWidget because the layout rules for a node are
// not easy to create in a composable way using the basic containers.
impl Widget for NodeWidget {
    fn layout(&mut self, ctx: &Context, parent_id: WidgetId, available: Vec2) -> Layout {
        todo!()
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        todo!()
    }

    fn min_size(&mut self, ctx: &Context, available: Vec2) -> Vec2 {
        todo!()
    }

    fn layout_hints(&self) -> LayoutHints {
        todo!()
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        events: &[Event],
    ) -> EventStatus {
        todo!()
    }
}
