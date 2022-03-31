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
/// boxing that a `UserData` requires. Instead we treat them as integers using
/// slotmap's `from_ffi` / `to_ffi` methods.
macro_rules! ids_from_to_lua {
    ($id_ty:ty) => {
        impl<'lua> ToLua<'lua> for $id_ty {
            fn to_lua(self, _lua: &'lua Lua) -> mlua::Result<mlua::Value<'lua>> {
                use slotmap::Key;
                Ok(mlua::Value::Number(keydata_to_float(self.data())))
            }
        }
        impl<'lua> FromLua<'lua> for $id_ty {
            fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua Lua) -> mlua::Result<Self> {
                match lua_value {
                    mlua::Value::Integer(id) => {
                        Ok(<$id_ty>::from(slotmap::KeyData::from_ffi(id as u64)))
                    }
                    mlua::Value::Number(id) => Ok(<$id_ty>::from(float_to_keydata(id))),
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

fn keydata_to_float(k: slotmap::KeyData) -> f64 {
    f64::from_le_bytes(k.as_ffi().to_le_bytes())
}

fn float_to_keydata(f: f64) -> slotmap::KeyData {
    slotmap::KeyData::from_ffi(u64::from_le_bytes(f.to_le_bytes()))
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
