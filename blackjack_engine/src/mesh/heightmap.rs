use glam::Vec2;
use mlua::UserData;
use noise::NoiseFn;

#[derive(Clone)]
pub struct HeightMap {
    inner: ndarray::Array2<f32>,
}

impl HeightMap {
    pub fn from_perlin(
        width: usize,
        height: usize,
        frequency: f32,
        offset: Vec2,
        amplitude: f32,
    ) -> HeightMap {
        let perlin = noise::Perlin::new();
        let inner = ndarray::Array2::from_shape_fn((width, height), |(x, y)| {
            let point = Vec2::new(x as f32 / frequency, y as f32 / frequency) + offset;
            perlin.get([point.x as f64, point.y as f64]) as f32 * amplitude
        });
        Self { inner }
    }
}

#[blackjack_macros::blackjack_lua_module]
mod lua_api {
    use crate::lua_engine::lua_stdlib::LVec3;

    use super::HeightMap;

    #[lua(under = "Blackjack")]
    pub fn heightmap_perlin(
        width: usize,
        height: usize,
        frequency: f32,
        offset: LVec3,
        amplitude: f32,
    ) -> HeightMap {
        HeightMap::from_perlin(width, height, frequency, offset.0.truncate(), amplitude)
    }
}

impl UserData for HeightMap {}
