use noise::NoiseFn;

use crate::prelude::halfedge::ChannelKey;

use super::*;
/// Vector types in Lua must be very lightweight. I have benchmarked the
/// overhead of having to cross the Rust <-> Lua boundary for every vector
/// operation and that is noticeably slower than simply using tables with x, y,
/// z fields to represent the vectors with a native lua library instead of using
/// userdata with a metatable.
macro_rules! def_vec_type {
    ($t:ident, $glam_t:ty, $($fields:ident),*) => {
        #[derive(Debug)]
        pub struct $t(pub $glam_t);
        impl<'lua> ToLua<'lua> for $t {
            fn to_lua(self, lua: &'lua Lua) -> mlua::Result<mlua::Value<'lua>> {
                let constructor = lua.globals()
                    .get::<_, Table>(stringify!($t))?.get::<_, mlua::Function>("new")?;
                constructor.call(($(self.0.$fields),*))
            }
        }
        impl<'lua> FromLua<'lua> for $t {
            fn from_lua(lua_value: mlua::Value<'lua>, _: &'lua Lua) -> mlua::Result<Self> {
                if let mlua::Value::Table(table) = lua_value {
                    Ok($t(<$glam_t>::new(
                        $(table.get(stringify!($fields))?),*
                    )))
                } else {
                    Err(mlua::Error::FromLuaConversionError {
                        from: lua_value.type_name(),
                        to: stringify!($t),
                        message: None,
                    })
                }
            }
        }
    };
}
def_vec_type!(Vec2, glam::Vec2, x, y);
def_vec_type!(Vec3, glam::Vec3, x, y, z);
def_vec_type!(Vec4, glam::Vec4, x, y, z, w);

impl UserData for SelectionExpression {}

#[derive(Clone, Debug)]
pub struct Path(pub std::path::PathBuf);
impl UserData for Path {}

impl UserData for HalfEdgeMesh {}

/// Vertex ids cross the Rust<->Lua boundary a lot, so we can't pay the price of
/// boxing that a `UserData` requires. Instead we treat them as integers using
/// slotmap's `from_ffi` / `to_ffi` methods.
macro_rules! ids_from_to_lua {
    ($id_ty:ty) => {
        impl<'lua> ToLua<'lua> for $id_ty {
            fn to_lua(self, lua: &'lua Lua) -> mlua::Result<mlua::Value<'lua>> {
                use slotmap::Key;
                self.data().as_ffi().to_lua(lua)
            }
        }
        impl<'lua> FromLua<'lua> for $id_ty {
            fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua Lua) -> mlua::Result<Self> {
                match lua_value {
                    mlua::Value::Integer(id) => {
                        Ok(<$id_ty>::from(slotmap::KeyData::from_ffi(id as u64)))
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

impl UserData for ChannelKeyType {}
impl UserData for ChannelValueType {}
pub fn load_channel_types(lua: &Lua) -> anyhow::Result<()> {
    let globals = lua.globals();

    let types = lua.create_table()?;
    types.set("VertexId", ChannelKeyType::VertexId)?;
    types.set("FaceId", ChannelKeyType::FaceId)?;
    types.set("HalfEdgeId", ChannelKeyType::HalfEdgeId)?;
    types.set("Vec2", ChannelValueType::Vec2)?;
    types.set("Vec3", ChannelValueType::Vec3)?;
    types.set("Vec4", ChannelValueType::Vec4)?;
    types.set("f32", ChannelValueType::f32)?;
    types.set("bool", ChannelValueType::bool)?;
    globals.set("Types", types)?;

    Ok(())
}

pub struct PerlinNoise(pub noise::Perlin);
impl UserData for PerlinNoise {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_3d", |_lua, this, v: Vec3| {
            Ok(this.0.get([v.0.x as f64, v.0.y as f64, v.0.z as f64]))
        })
    }
}
