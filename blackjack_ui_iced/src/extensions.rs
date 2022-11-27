use iced::Color;

pub trait ColorExt {
    fn as_color(&self) -> Color;

    fn mul(&self, value: f32) -> Color {
        let c = self.as_color();
        Color {
            r: c.r * value,
            g: c.g * value,
            b: c.b * value,
            a: c.a,
        }
    }

    fn add(&self, value: f32) -> Color {
        let c = self.as_color();
        Color {
            r: c.r + value,
            g: c.g + value,
            b: c.b + value,
            a: c.a,
        }
    }

    fn with_alpha(&self, a: f32) -> Color {
        let c = self.as_color();
        Color {
            r: c.r,
            g: c.g,
            b: c.b,
            a,
        }
    }
}

impl ColorExt for Color {
    fn as_color(&self) -> Color {
        *self
    }
}
