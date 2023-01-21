// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use blackjack_commons::utils::transmute_vec;
use noise::NoiseFn;

use super::*;

#[derive(Debug)]
#[repr(transparent)]
pub struct LVec3(pub glam::Vec3);
impl<'lua> ToLua<'lua> for LVec3 {
    fn to_lua(self, _lua: &'lua Lua) -> mlua::Result<mlua::Value<'lua>> {
        Ok(mlua::Value::Vector(self.0.x, self.0.y, self.0.z))
    }
}
impl<'lua> FromLua<'lua> for LVec3 {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua Lua) -> mlua::Result<Self> {
        match lua_value {
            mlua::Value::Vector(x, y, z) => Ok(LVec3(glam::Vec3::new(x, y, z))),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: lua_value.type_name(),
                to: "Vec3",
                message: None,
            }),
        }
    }
}
impl From<glam::Vec3> for LVec3 {
    fn from(v: glam::Vec3) -> Self {
        Self(v)
    }
}
impl LVec3 {
    /// Unwraps all the internal Vec<LVec3> values.
    pub fn cast_vector(this: Vec<LVec3>) -> Vec<glam::Vec3> {
        // SAFETY: LVec3 is marked as #[repr(transparent)] and contains a
        // single glam::Vec3. Both types are copy and have no Drop logic.
        unsafe { transmute_vec(this) }
    }
}

/// Vertex ids cross the Rust<->Lua boundary a lot, so we can't pay the price of
/// boxing that a `UserData` requires. Instead we use LightUserData by casting
/// the slotmap key to u64, and then to a pointer.
///
/// SAFETY: Note that the cast to pointer is completely safe, since we're never
/// really dereferencing that pointer. It's just the mechanism Lua gives us to
/// store an opaque u64 value which *could* be a pointer but in our case just
/// isn't.
macro_rules! ids_from_to_lua {
    ($id_ty:ty) => {
        impl<'lua> ToLua<'lua> for $id_ty {
            fn to_lua(self, _lua: &'lua Lua) -> mlua::Result<mlua::Value<'lua>> {
                use slotmap::Key;
                Ok(mlua::Value::LightUserData(keydata_to_lightdata(
                    self.data(),
                )))
            }
        }
        impl<'lua> FromLua<'lua> for $id_ty {
            fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua Lua) -> mlua::Result<Self> {
                match lua_value {
                    mlua::Value::LightUserData(lud) => {
                        Ok(<$id_ty>::from(ligthdata_to_keydata(lud)))
                    }
                    _ => Err(mlua::Error::FromLuaConversionError {
                        from: lua_value.type_name(),
                        to: stringify!($id_ty),
                        message: None,
                    }),
                }
            }
        }
    };
}
ids_from_to_lua!(VertexId);
ids_from_to_lua!(FaceId);
ids_from_to_lua!(HalfEdgeId);

fn keydata_to_lightdata(k: slotmap::KeyData) -> mlua::LightUserData {
    mlua::LightUserData(k.as_ffi() as *mut std::ffi::c_void)
}
fn ligthdata_to_keydata(d: mlua::LightUserData) -> slotmap::KeyData {
    slotmap::KeyData::from_ffi(d.0 as u64)
}

impl UserData for ChannelKeyType {}
impl UserData for ChannelValueType {}

pub struct PerlinNoise(pub noise::Perlin);

#[blackjack_macros::blackjack_lua_module]
mod perlin_noise {
    use super::*;

    /// Constructs a new PerlinNoise
    #[lua(under = "PerlinNoise")]
    pub fn new() -> PerlinNoise {
        PerlinNoise(noise::Perlin::new())
    }

    #[lua_impl]
    impl PerlinNoise {
        /// Sample a standard 3d perlin noise function at coordinates `(x, y, z)`
        #[lua]
        pub fn get_3d(&self, x: f64, y: f64, z: f64) -> f64 {
            // NOTE: Noise crate crashes when given weird numbers. We can't
            // afford to crash when weird numbers are sent from Lua, so we need
            // to add this guard here.
            if x.is_finite() && y.is_finite() && z.is_finite() {
                self.0.get([x, y, z])
            } else {
                f64::NAN
            }
        }
    }
}

#[blackjack_macros::blackjack_lua_module]
mod vector_math {
    use super::*;
    use glam::Quat;

    /// Return vector `v` rotated around given `axis` and `angle` (in radians).
    #[lua(under = "NativeMath")]
    pub fn rotate_around_axis(v: LVec3, axis: LVec3, angle: f32) -> LVec3 {
        let q = Quat::from_axis_angle(axis.0.normalize(), angle);
        LVec3(q * v.0)
    }

    /// Returns the `cross` product of vectors `v` and `v2`.
    #[lua(under = "NativeMath")]
    pub fn cross(v: LVec3, v2: LVec3) -> LVec3 {
        LVec3(v.0.cross(v2.0))
    }
}
