use std::{cell::RefCell, rc::Rc};

use noise::NoiseFn;

use crate::prelude::halfedge::{ChannelKey, ChannelValue, DynChannel};

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
//def_vec_type!(Vec3, glam::Vec3, x, y, z);
def_vec_type!(Vec4, glam::Vec4, x, y, z, w);

pub struct Vec3(pub glam::Vec3);
impl<'lua> ToLua<'lua> for Vec3 {
    fn to_lua(self, lua: &'lua Lua) -> mlua::Result<mlua::Value<'lua>> {
        Ok(mlua::Value::Vector(self.0.x, self.0.y, self.0.z))
    }
}
impl<'lua> FromLua<'lua> for Vec3 {
    fn from_lua(lua_value: mlua::Value<'lua>, lua: &'lua Lua) -> mlua::Result<Self> {
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

pub struct SharedChannel(pub Rc<RefCell<dyn DynChannel>>);
impl Clone for SharedChannel {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl UserData for HalfEdgeMesh {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method(
            "get_channel",
            |_lua, this, (kty, vty, name): (ChannelKeyType, ChannelValueType, String)| {
                Ok(SharedChannel(
                    this.channels
                        .channel_rc_dyn(kty, vty, &name)
                        .map_lua_err()?,
                ))
            },
        );
        methods.add_method(
            "get_channel_2",
            |lua, this, (kty, vty, name): (ChannelKeyType, ChannelValueType, String)| {
                profiling::scope!("get_channel2");
                use slotmap::Key;
                let conn = this.read_connectivity();
                let keys: Box<dyn Iterator<Item = u64>> = match kty {
                    ChannelKeyType::VertexId => {
                        Box::new(conn.iter_vertices().map(|(v_id, _)| v_id.data().as_ffi()))
                    }
                    ChannelKeyType::FaceId => {
                        Box::new(conn.iter_faces().map(|(f_id, _)| f_id.data().as_ffi()))
                    }
                    ChannelKeyType::HalfEdgeId => {
                        Box::new(conn.iter_halfedges().map(|(h_id, _)| h_id.data().as_ffi()))
                    }
                };
                Ok(this
                    .channels
                    .dyn_read_channel_by_name(kty, vty, &name)
                    .map_lua_err()?
                    .to_table(keys, lua))
            },
        );
        methods.add_method("set_channel", |lua, this, (kty, vty, name, table)| {
            profiling::scope!("set_channel");
            use slotmap::Key;
            let name: String = name;
            let conn = this.read_connectivity();
            let keys: Box<dyn Iterator<Item = u64>> = match kty {
                ChannelKeyType::VertexId => {
                    Box::new(conn.iter_vertices().map(|(v_id, _)| v_id.data().as_ffi()))
                }
                ChannelKeyType::FaceId => {
                    Box::new(conn.iter_faces().map(|(f_id, _)| f_id.data().as_ffi()))
                }
                ChannelKeyType::HalfEdgeId => {
                    Box::new(conn.iter_halfedges().map(|(h_id, _)| h_id.data().as_ffi()))
                }
            };
            this.channels
                .dyn_write_channel_by_name(kty, vty, &name)
                .map_lua_err()?
                .set_from_table(keys, lua, table)
                .map_lua_err()
        });
        methods.add_method_mut(
            "ensure_channel",
            |_lua, this, (kty, vty, name): (ChannelKeyType, ChannelValueType, String)| {
                let _ = this.channels.ensure_channel_dyn(kty, vty, &name);
                Ok(SharedChannel(
                    // TODO: This needlesly recomputes the name->ch_id correspondence
                    this.channels
                        .channel_rc_dyn(kty, vty, &name)
                        .map_lua_err()?,
                ))
            },
        );
        methods.add_method_mut("iter_vertices", |lua, this, ()| {
            let vertices: Vec<VertexId> = this
                .read_connectivity()
                .iter_vertices()
                .map(|(id, _)| id)
                .collect();
            let mut i = 0;
            lua.create_function_mut(move |lua, ()| {
                let val = if i < vertices.len() {
                    vertices[i].to_lua(lua)?
                } else {
                    mlua::Value::Nil
                };
                i += 1;
                Ok(val)
            })
        });
        methods.add_method("clone", |_lua, this, ()| Ok(this.clone()));
    }
}

#[test]
pub fn minibench() {
    let lua = Lua::new();

    let v = vec![42; 3000];
    let mut i = 0;
    let f = lua
        .create_function_mut(move |lua, ()| {
            println!("Iteration {i}");
            let val = if i < v.len() {
                mlua::Value::Number(v[i] as f64)
            } else {
                mlua::Value::Nil
            };
            i += 1;
            Ok(val)
        })
        .unwrap();

    let main = lua
        .load("return function(f) for i in f do print(i) end end")
        .call::<_, mlua::Function>(())
        .unwrap();

    main.call::<_, ()>(f);
}

impl UserData for SharedChannel {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(
            mlua::MetaMethod::NewIndex,
            |lua, this, (key, val): (mlua::Value, mlua::Value)| {
                this.0.borrow_mut().set_lua(lua, key, val).map_lua_err()?;
                Ok(())
            },
        );
        methods.add_meta_method(mlua::MetaMethod::Index, |lua, this, key: mlua::Value| {
            let value = this.0.borrow().get_lua(lua, key).map_lua_err()?;
            Ok(value.clone())
        });
        methods.add_meta_method(
            mlua::MetaMethod::NewIndex,
            |lua, this, (key, val): (mlua::Value, mlua::Value)| {
                this.0.borrow_mut().set_lua(lua, key, val).map_lua_err()?;
                Ok(())
            },
        );
    }
}

/// Vertex ids cross the Rust<->Lua boundary a lot, so we can't pay the price of
/// boxing that a `UserData` requires. Instead we treat them as integers using
/// slotmap's `from_ffi` / `to_ffi` methods.
macro_rules! ids_from_to_lua {
    ($id_ty:ty) => {
        impl<'lua> ToLua<'lua> for $id_ty {
            fn to_lua(self, lua: &'lua Lua) -> mlua::Result<mlua::Value<'lua>> {
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
pub fn load_channel_types(lua: &Lua) -> anyhow::Result<()> {
    let globals = lua.globals();

    let types = lua.create_table()?;
    types.set("VertexId", ChannelKeyType::VertexId)?;
    types.set("FaceId", ChannelKeyType::FaceId)?;
    types.set("HalfEdgeId", ChannelKeyType::HalfEdgeId)?;
    types.set("Vec3", ChannelValueType::Vec3)?;
    types.set("f32", ChannelValueType::f32)?;
    globals.set("Types", types)?;

    Ok(())
}

pub struct PerlinNoise(pub noise::Perlin);
impl UserData for PerlinNoise {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_3d", |_lua, this, (x, y, z): (f64, f64, f64)| {
            //Ok(this.0.get([v.0.x as f64, v.0.y as f64, v.0.z as f64]))
            Ok(this.0.get([x, y, z]))
        })
    }
}
