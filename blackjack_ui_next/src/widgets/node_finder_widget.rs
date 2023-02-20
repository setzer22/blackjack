use epaint::{ahash::HashMap, emath::Align2, RectShape, Rounding};
use guee::{
    callback::PollToken,
    prelude::{guee_derives::Builder, *},
};

use crate::{graph_editor::GraphEditor, pallette};

#[derive(Default)]
pub struct NodeFinder {
    search_box_contents: String,
}

impl NodeFinder {
    pub fn view(&self, op_names: Vec<String>) -> DynWidget {
        let search_box = TextEdit::new(
            IdGen::key("node_finder_search"),
            self.search_box_contents.clone(),
        )
        .padding(Vec2::new(5.0, 5.0))
        .layout_hints(LayoutHints::fill_horizontal())
        .on_changed(|this: &mut NodeFinder, new| {
            this.search_box_contents = new;
        })
        .build();

        let buttons = op_names
            .iter()
            .filter(|op_name| {
                self.search_box_contents.is_empty()
                    || op_name
                        .to_lowercase()
                        .contains(&self.search_box_contents.to_lowercase())
            })
            .map(|op_name| {
                let op_name = op_name.clone(); // need to move into closure
                Button::with_label(&op_name)
                    .hints(LayoutHints::fill_horizontal())
                    .align_contents(Align2::LEFT_CENTER)
                    .padding(Vec2::new(3.0, 3.0))
                    .on_click(move |graph: &mut GraphEditor, _| {
                        println!("New node {op_name}");
                    })
                    .build()
            })
            .collect();

        let button_container = MarginContainer::new(
            IdGen::key("margin"),
            BoxContainer::vertical(IdGen::key("buttons"), buttons)
                .layout_hints(LayoutHints::fill())
                .build(),
        )
        .margin(Vec2::new(10.0, 10.0))
        .build();

        let contents = BoxContainer::vertical(
            IdGen::key("node_finder"),
            vec![search_box, button_container],
        )
        .layout_hints(LayoutHints::fill())
        .build();

        SizedContainer::new(
            CustomDrawContainer::new(contents)
                .pre_draw(|painter, layout| {
                    painter.rect(RectShape {
                        rect: layout.bounds.expand2(Vec2::new(1.0, 1.0)),
                        rounding: Rounding::same(1.0),
                        fill: pallette().widget_bg_dark,
                        stroke: Stroke::NONE,
                    });
                })
                .build(),
            Vec2::new(300.0, 500.0),
        )
        .build()
    }
}
