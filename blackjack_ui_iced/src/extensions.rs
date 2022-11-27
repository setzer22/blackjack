use iced::{Color, Point, Vector, Rectangle};

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

pub trait PointExt {
    fn as_point(&self) -> Point;
    fn to_vector(&self) -> Vector {
        let p = self.as_point();
        Vector { x: p.x, y: p.y }
    }
}

impl PointExt for Point {
    fn as_point(&self) -> Point {
        *self
    }
}

pub trait VectorExt {
    fn as_vector(&self) -> Vector;
    fn to_point(&self) -> Point {
        let v = self.as_vector();
        Point { x: v.x, y: v.y }
    }
}

impl VectorExt for Vector {
    fn as_vector(&self) -> Vector {
        *self
    }
}

pub trait RectangleExt {
    fn as_rectangle(&self) -> Rectangle;
    fn top_left(&self) -> Point {
        let r = self.as_rectangle();
        Point { x: r.x, y: r.y }
    }
    fn top_right(&self) -> Point {
        let r = self.as_rectangle();
        Point { x: r.x + r.width, y: r.y }
    }
    fn bottom_left(&self) -> Point {
        let r = self.as_rectangle();
        Point { x: r.x, y: r.y + r.height }
    }
    fn bottom_right(&self) -> Point {
        let r = self.as_rectangle();
        Point { x: r.x + r.width, y: r.y + r.height }
    }
    fn center_left(&self) -> Point {
        let r = self.as_rectangle();
        Point { x: r.x, y: r.y + r.height * 0.5 }
    }
    fn center_right(&self) -> Point {
        let r = self.as_rectangle();
        Point { x: r.x + r.width, y: r.y + r.height * 0.5 }
    }
    fn top_center(&self) -> Point {
        let r = self.as_rectangle();
        Point { x: r.x + r.width * 0.5, y: r.y }
    }
    fn bottom_center(&self) -> Point {
        let r = self.as_rectangle();
        Point { x: r.x + r.width * 0.5, y: r.y + r.height }
    }
}

impl RectangleExt for Rectangle {
    fn as_rectangle(&self) -> Rectangle {
        *self
    }
}
