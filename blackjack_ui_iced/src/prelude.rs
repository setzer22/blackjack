/// Iced makes heavy use of generics in all its APIs to be more customizable.
/// While nice, this crate is not a library, so we remove most of the generics
/// via type aliases to keep our function signatures clean.
mod no_generics {
    pub type BjkUiRenderer = iced_graphics::Renderer<iced_wgpu::Backend, crate::theme::BjkUiTheme>;

    pub type BjkUiElement<'a> = iced::Element<'a, crate::BjkUiMessage, BjkUiRenderer>;

    macro_rules! BjkWidget {
        () => {

        };
    }
}
pub use no_generics::*;

mod theming {
    pub use crate::theme::*;
}
pub use theming::*;

pub use crate::extensions::*;
pub use crate::BjkUiMessage;

pub mod iced_prelude {
    pub use iced::Color;
    pub use iced::Length;
    pub use iced::Point;
    pub use iced::Vector;
    pub use iced::Rectangle;
    pub use iced::Size;
    pub use iced::Background;
    pub use iced_native::renderer::Quad;
    pub use iced_native::layout::Limits;
    pub use iced_native::layout::Layout;
    pub type LayoutNode = iced_native::layout::Node;
    pub type WidgetTree = iced_native::widget::Tree;

    pub type RendererStyle = iced_native::renderer::Style;

}
