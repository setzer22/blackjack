use std::ffi::c_void;

use noise::NoiseFn;

use super::*;

pub struct Vec3(pub glam::Vec3);
impl<'lua> ToLua<'lua> for Vec3 {
    fn to_lua(self, _lua: &'lua Lua) -> mlua::Result<mlua::Value<'lua>> {
        Ok(mlua::Value::Vector(self.0.x, self.0.y, self.0.z))
    }
}
impl<'lua> FromLua<'lua> for Vec3 {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua Lua) -> mlua::Result<Self> {
        match lua_value {
            mlua::Value::Vector(x, y, z) => Ok(Vec3(glam::Vec3::new(x, y, z))),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: lua_value.type_name(),
                to: "Vec3",
                message: None,
            }),
        }
    }
}

impl UserData for SelectionExpression {}

#[derive(Clone, Debug)]
pub struct Path(pub std::path::PathBuf);
impl UserData for Path {}

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
impl UserData for PerlinNoise {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_3d", |_lua, this, (x, y, z): (f64, f64, f64)| {
            Ok(this.0.get([x, y, z]))
        })
    }
}
