// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::Result;
use glam::{Vec2, Vec3};
use noise::NoiseFn;

use crate::prelude::VertexIndexBuffers;

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
        let frequency = frequency.max(0.01);
        let perlin = noise::Perlin::new();
        let inner = ndarray::Array2::from_shape_fn((width, height), |(x, y)| {
            let point = Vec2::new(x as f32 / frequency, y as f32 / frequency) + offset;
            perlin.get([point.x as f64, point.y as f64]) as f32 * amplitude
        });

        Self { inner }
    }

    pub fn from_lua_fn(width: usize, height: usize, f: mlua::Function) -> Result<HeightMap> {
        // We can't jump out of the closure, so if there's an error we store it
        // here and return at the end.
        let mut error = None;

        let inner = ndarray::Array2::from_shape_fn((width, height), |(i, j)| {
            match f.call::<_, f32>((i, j)).map_err(|err| anyhow::anyhow!(err)) {
                Ok(height) => height,
                Err(err) => {
                    error = Some(err);
                    0.0
                }
            }
        });

        if let Some(error) = error {
            Err(error)
        } else {
            Ok(Self { inner })
        }
    }

    pub fn generate_triangle_buffers(&self) -> VertexIndexBuffers {
        // If the terrain is too small to compute normals, return an empty buffer
        if self.inner.ncols() < 4 || self.inner.nrows() < 4 {
            return VertexIndexBuffers {
                positions: vec![],
                normals: vec![],
                indices: vec![],
            };
        }

        let scale = 0.05;

        let mut positions = vec![];
        let mut indices = vec![];
        let mut normals = vec![];

        // Iterate 4x4 windows.
        //
        // NOTE: ndarray should have utilities for this, but in practice it
        // doesn't because it doesn't let you check the indices while iterating
        for i in 1..self.inner.nrows() - 1 {
            for j in 1..self.inner.ncols() - 1 {
                // SAFETY: Always in bounds
                let point = unsafe {
                    let height = &self.inner;
                    let y = *height.uget((i, j));
                    let x = j as f32 * scale;
                    let z = i as f32 * scale;

                    Vec3::new(x, y, z)
                };

                positions.push(point);

                // SAFETY: Always in bounds due to loop bounds above
                let normal = unsafe {
                    let height = &self.inner;
                    Vec3::new(
                        *height.uget((i, j - 1)) - *height.uget((i, j + 1)),
                        2.0 * scale,
                        *height.uget((i - 1, j)) - *height.uget((i + 1, j)),
                    )
                    .normalize()
                };
                normals.push(normal);
            }
        }

        // We discarded the edge points so we could compute the normals, so now
        let nrows = self.inner.nrows() - 2;
        let ncols = self.inner.ncols() - 2;

        for i in 0..nrows - 1 {
            for j in 0..ncols - 1 {
                let a = j + i * ncols;
                let b = a + 1;
                let c = a + ncols;
                let d = a + ncols + 1;
                indices.extend([a, c, b, b, c, d].map(|x| x as u32))
            }
        }

        VertexIndexBuffers {
            positions,
            normals,
            indices,
        }
    }
}

#[blackjack_macros::blackjack_lua_module]
mod lua_api {
    use super::*;
    use crate::lua_engine::lua_stdlib::LVec3;

    use super::HeightMap;

    /// Builds a heightmap with a grid of `width` times `height` filled with
    /// perlin noise with given parameters.
    #[lua(under = "HeightMap")]
    pub fn from_perlin(
        width: usize,
        height: usize,
        frequency: f32,
        offset: LVec3,
        amplitude: f32,
    ) -> HeightMap {
        HeightMap::from_perlin(width, height, frequency, offset.0.truncate(), amplitude)
    }

    /// Builds a heightmap with a grid of `width` times `height` filled with
    /// perlin noise, where each cell is filled with the result of running the
    /// function `f`. The function is called with the (i, j) coordinates of the
    /// current cell.
    #[lua(under = "HeightMap")]
    pub fn from_fn(width: usize, height: usize, f: mlua::Function) -> Result<HeightMap> {
        HeightMap::from_lua_fn(width, height, f)
    }

    #[lua_impl]
    impl HeightMap {
        /// Returns the width of this heightmap
        #[lua]
        fn width(&self) -> usize {
            self.inner.dim().0
        }

        /// Returns the height of this heightmap
        #[lua]
        fn height(&self) -> usize {
            self.inner.dim().1
        }
    }
}
