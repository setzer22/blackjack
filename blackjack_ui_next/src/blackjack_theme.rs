use epaint::Color32;
use guee::{base_widgets::split_pane_container::SplitPaneContainerStyle, prelude::*};

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
    theme.set_style::<Button>(ButtonStyle::with_base_colors(
        pallette.widget_bg,
        Stroke::NONE,
        1.1,
        1.3,
    ));

    theme.set_style::<SplitPaneContainer>(SplitPaneContainerStyle::new(pallette.widget_fg_dark));

    theme.text_color = pallette.widget_fg;

    theme
}
