use epaint::{ahash::HashMap, emath::Align2, RectShape, Rounding};
use guee::{
    callback::PollToken,
    prelude::{guee_derives::Builder, *},
};

use crate::pallette;

#[derive(Builder)]
#[builder(widget, skip_new)]
pub struct NodeFinderWidget {
    pub id: IdGen,

    /// Inner widget
    pub inner: Option<DynWidget>,

    pub op_names: Vec<String>,

    /// Returns the op_name for the selected node, once the user clicks it.
    #[builder(callback)]
    pub on_selection: Option<Callback<String>>,

    pub button_poll_tks: Vec<PollToken<()>>,
    pub search_box_tk: Option<PollToken<String>>,
}

impl NodeFinderWidget {
    pub fn new(id: IdGen, op_names: Vec<String>) -> Self {
        Self {
            id,
            op_names,
            on_selection: None,
            // These are filled later
            inner: None,
            button_poll_tks: Default::default(),
            search_box_tk: Default::default(),
        }
    }
}

pub struct NodeFinderWidgetState {
    pub search_box_contents: String,
}

impl Widget for NodeFinderWidget {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        _available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        if force_shrink {
            SizeHint::ignore_force_warning("NodeFinderWidget");
        }

        let widget_id = self.id.resolve(parent_id);
        let state = ctx.memory.get_or(
            widget_id,
            NodeFinderWidgetState {
                search_box_contents: "".into(),
            },
        );

        self.inner = {
            let search_box = {
                let mut search_box = TextEdit::new(
                    IdGen::key("node_finder_search"),
                    state.search_box_contents.clone(),
                )
                .padding(Vec2::new(5.0, 5.0))
                .layout_hints(LayoutHints::fill_horizontal());

                let (search_cb, search_tk) = ctx.create_internal_callback();
                search_box.on_changed = Some(search_cb);
                self.search_box_tk = Some(search_tk);
                search_box.build()
            };

            let mut buttons = vec![];
            let mut button_poll_tokens = vec![];
            for op_name in self.op_names.iter() {
                if state.search_box_contents.is_empty()
                    || op_name
                        .to_lowercase()
                        .contains(&state.search_box_contents.to_lowercase())
                {
                    let (cb, token) = ctx.create_internal_callback::<()>();

                    let mut button = Button::with_label(op_name)
                        .hints(LayoutHints::fill_horizontal())
                        .align_contents(Align2::LEFT_CENTER)
                        .padding(Vec2::new(3.0, 3.0));
                    button.on_click = Some(cb);

                    buttons.push(button.build());
                    button_poll_tokens.push(token);
                }
            }

            let button_container = MarginContainer::new(
                IdGen::key("margin"),
                BoxContainer::vertical(IdGen::key("buttons"), buttons)
                    .layout_hints(LayoutHints::fill())
                    .build(),
            )
            .margin(Vec2::new(10.0, 10.0))
            .build();

            let node_finder = BoxContainer::vertical(
                IdGen::key("node_finder"),
                vec![search_box, button_container],
            )
            .layout_hints(LayoutHints::fill())
            .build();

            Some(node_finder)
        };

        // TODO: Hardcoded size
        let size = Vec2::new(180.0, 500.0);

        drop(state);

        let inner_layout = self
            .inner
            .as_mut()
            .unwrap()
            .widget
            .layout(ctx, widget_id, size, false);

        Layout::with_children(widget_id, size, vec![inner_layout])
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        ctx.painter().rect(RectShape {
            rect: layout.bounds.expand2(Vec2::new(1.0, 1.0)),
            rounding: Rounding::same(1.0),
            fill: pallette().widget_bg_dark,
            stroke: Stroke::NONE,
        });

        self.inner
            .as_mut()
            .unwrap()
            .widget
            .draw(ctx, &layout.children[0]);
    }

    fn layout_hints(&self) -> LayoutHints {
        LayoutHints::shrink()
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        events: &[Event],
    ) -> EventStatus {
        let status = self.inner.as_mut().unwrap().widget.on_event(
            ctx,
            &layout.children[0],
            cursor_position,
            events,
        );

        // Check if any of the buttons fired
        for (idx, tk) in self.button_poll_tks.iter().copied().enumerate() {
            if ctx.poll_callback_result(tk).is_some() {
                if let Some(on_selection) = self.on_selection.take() {
                    ctx.dispatch_callback(on_selection, self.op_names[idx].clone())
                }
            }
        }

        if let Some(changed_string) = ctx.poll_callback_result(self.search_box_tk.unwrap()) {
            let mut state = ctx
                .memory
                .get_mut::<NodeFinderWidgetState>(layout.widget_id);
            state.search_box_contents = changed_string;
        }

        status
    }
}
