use iced::{Background, Color, Font, Vector};

pub mod color_hex_utils;
use color_hex_utils::*;

use crate::{prelude::*, BjkUiMessage};

pub struct BjkUiTheme {
    pub text_color: Color,
    pub widget_bg: Color,
    pub widget_bg_light: Color,
    pub widget_bg_dark: Color,
    pub widget_fg: Color,
    pub widget_fg_light: Color,
    pub widget_fg_dark: Color,
    pub accent: Color,
    pub background: Color,
    pub background_dark: Color,
}

impl BjkUiTheme {
    pub const DEFAULT_TEXT_SIZE: u16 = 17;
    pub const FONT_REGULAR: Font = Font::External {
        name: "NunitoSans-Bold.ttf",
        bytes: include_bytes!("../resources/fonts/NunitoSans-Regular.ttf"),
    };
    pub const FONT_LIGHT: Font = Font::External {
        name: "NunitoSans-Light.ttf",
        bytes: include_bytes!("../resources/fonts/NunitoSans-Regular.ttf"),
    };
    pub const FONT_BOLD: Font = Font::External {
        name: "NunitoSans-Bold.ttf",
        bytes: include_bytes!("../resources/fonts/NunitoSans-Regular.ttf"),
    };
    pub const FONT_EXTRA_BOLD: Font = Font::External {
        name: "NunitoSans-ExtraBold.ttf",
        bytes: include_bytes!("../resources/fonts/NunitoSans-Regular.ttf"),
    };
}

impl Default for BjkUiTheme {
    fn default() -> Self {
        Self {
            text_color: color_from_hex("#e3e3e3").unwrap(),
            widget_bg_light: color_from_hex("#464646").unwrap(),
            widget_bg: color_from_hex("#303030").unwrap(),
            widget_bg_dark: color_from_hex("#2c2c2c").unwrap(),
            widget_fg: color_from_hex("#c0c0c0").unwrap(),
            widget_fg_light: color_from_hex("#dddddd").unwrap(),
            widget_fg_dark: color_from_hex("#9b9b9b").unwrap(),
            accent: color_from_hex("#b43e3e").unwrap(),
            background: color_from_hex("#303030").unwrap(),
            background_dark: color_from_hex("#1d1d1d").unwrap(),
        }
    }
}

impl iced_style::button::StyleSheet for BjkUiTheme {
    type Style = ();

    fn active(&self, _style: &Self::Style) -> iced_style::button::Appearance {
        iced_style::button::Appearance {
            shadow_offset: Vector::default(),
            background: Some(Background::Color(self.widget_bg)),
            border_radius: 2.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: self.widget_fg,
        }
    }

    fn hovered(&self, style: &Self::Style) -> iced_style::button::Appearance {
        let active = self.active(style);

        iced_style::button::Appearance {
            background: Some(Background::Color(self.widget_bg.mul(1.15))),
            ..active
        }
    }

    fn pressed(&self, style: &Self::Style) -> iced_style::button::Appearance {
        iced_style::button::Appearance {
            background: Some(Background::Color(self.widget_bg.mul(1.4))),
            ..self.active(style)
        }
    }

    fn disabled(&self, style: &Self::Style) -> iced_style::button::Appearance {
        let active = self.active(style);

        iced_style::button::Appearance {
            background: active.background.map(|background| match background {
                Background::Color(color) => Background::Color(Color {
                    a: color.a * 0.5,
                    ..color
                }),
            }),
            text_color: Color {
                a: active.text_color.a * 0.5,
                ..active.text_color
            },
            ..active
        }
    }
}

impl iced::application::StyleSheet for BjkUiTheme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> iced::application::Appearance {
        iced::application::Appearance {
            background_color: self.background_dark,
            text_color: self.text_color,
        }
    }
}

#[derive(Default, Clone)]
pub enum BjkContainerStyle {
    #[default]
    Transparent,
    Pane,
}

impl iced_style::container::StyleSheet for BjkUiTheme {
    type Style = BjkContainerStyle;

    fn appearance(&self, style: &Self::Style) -> iced_style::container::Appearance {
        match style {
            BjkContainerStyle::Transparent => Default::default(),
            BjkContainerStyle::Pane => iced_style::container::Appearance {
                background: Some(Background::Color(self.background)),
                border_width: 2.0,
                border_color: self.widget_bg_light,
                text_color: None,
                border_radius: 2.0,
            },
        }
    }
}

impl iced_style::text::StyleSheet for BjkUiTheme {
    type Style = ();

    fn appearance(&self, _style: Self::Style) -> iced_style::text::Appearance {
        // Will inherit the default color by default
        Default::default()
    }
}

impl iced_style::pane_grid::StyleSheet for BjkUiTheme {
    type Style = ();

    fn picked_split(&self, _style: &Self::Style) -> Option<iced_style::pane_grid::Line> {
        Some(iced_style::pane_grid::Line {
            color: self.widget_fg,
            width: 2.0,
        })
    }

    fn hovered_split(&self, _style: &Self::Style) -> Option<iced_style::pane_grid::Line> {
        Some(iced_style::pane_grid::Line {
            color: self.widget_bg_light,
            width: 2.0,
        })
    }
}

pub fn button<'a>(
    content: impl Into<String>,
) -> iced::widget::Button<'a, BjkUiMessage, BjkUiRenderer> {
    iced::widget::button(text(content))
        .padding(2)
        .on_press(BjkUiMessage::Dummy)
}

pub fn text<'a>(s: impl Into<String>) -> iced::widget::Text<'a, BjkUiRenderer> {
    iced::widget::text(s.into()).size(BjkUiTheme::DEFAULT_TEXT_SIZE)
}

pub fn row(content: Vec<BjkUiElement<'_>>) -> iced::widget::Row<'_, BjkUiMessage, BjkUiRenderer> {
    iced::widget::row(content)
}

pub fn column(
    content: Vec<BjkUiElement<'_>>,
) -> iced::widget::Column<'_, BjkUiMessage, BjkUiRenderer> {
    iced::widget::column(content)
}

pub fn container<'a>(
    content: impl Into<BjkUiElement<'a>>,
) -> iced::widget::Container<'a, BjkUiMessage, BjkUiRenderer> {
    iced::widget::container(content.into())
}

pub fn h_spacer() -> iced::widget::Space {
    iced::widget::Space::new(iced::Length::Fill, iced::Length::Shrink)
}

pub fn v_spacer() -> iced::widget::Space {
    iced::widget::Space::new(iced::Length::Fill, iced::Length::Shrink)
}

pub fn empty_space() -> iced::widget::Space {
    iced::widget::Space::new(iced::Length::Shrink, iced::Length::Shrink)
}
