use epaint::{Color32, Rounding};
use guee::{
    base_widgets::{
        menubar_button::{MenubarButton, MenubarButtonStyle},
        split_pane_container::SplitPaneContainerStyle,
    },
    prelude::*,
};

pub struct BlackjackPallette {
    pub widget_bg: Color32,
    pub widget_bg_light: Color32,
    pub widget_bg_dark: Color32,

    pub widget_fg: Color32,
    pub widget_fg_light: Color32,
    pub widget_fg_dark: Color32,

    pub accent: Color32,

    pub background: Color32,
    pub background_dark: Color32,
}

#[inline(always)]
pub fn pallette() -> BlackjackPallette {
    BlackjackPallette {
        widget_bg: color!("#303030"),
        widget_bg_light: color!("#464646"),
        widget_bg_dark: color!("#2c2c2c"),

        widget_fg: color!("#c0c0c0"),
        widget_fg_light: color!("#dddddd"),
        widget_fg_dark: color!("#9b9b9b"),

        accent: color!("#b43e3e"),

        background: color!("#191919"),
        background_dark: color!("#1d1d1d"),
    }
}

pub fn blackjack_theme() -> Theme {
    let mut theme = Theme::new_empty();
    let pallette = pallette();
    let button_style = ButtonStyle::with_base_colors(pallette.widget_bg, Stroke::NONE, 1.1, 1.3);
    theme.set_style::<Button>(button_style);

    let mut menubar_button_style =
        ButtonStyle::with_base_colors(pallette.accent, Stroke::NONE, 1.1, 1.3);
    menubar_button_style.idle_fill = pallette.widget_bg;
    theme.set_style::<MenubarButton>(MenubarButtonStyle {
        outer_button: menubar_button_style.clone().rounding(Rounding::same(0.0)),
        inner_button: menubar_button_style,
        menu_fill: pallette.widget_bg_dark,
        menu_stroke: Stroke::new(1.0, pallette.widget_bg_light),
    });

    theme.set_style::<SplitPaneContainer>(SplitPaneContainerStyle::new(pallette.widget_fg_dark));

    theme.text_color = pallette.widget_fg;

    theme
}
