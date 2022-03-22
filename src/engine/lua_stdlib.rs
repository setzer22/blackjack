use std::collections::HashMap;

use mlua::{AnyUserData, FromLua, Lua, Table, ToLua, UserData};

use crate::{
    engine::ToLuaError,
    prelude::{
        graph::node_templates2::NodeDefinition, selection::SelectionExpression, HalfEdgeMesh,
    },
};

pub struct LuaRuntime {
    pub lua: Lua,
    pub node_definitions: HashMap<String, NodeDefinition>,
}

/// Vector types in Lua must be very lightweight. I have benchmarked the
/// overhead of having to cross the Rust <-> Lua boundary for every vector
/// operation and that is noticeably slower than simply using tables with x, y,
/// z fields to represent the vectors with a native lua library instead of using
/// userdata with a metatable.
macro_rules! def_vec_type {
    ($t:ident, $glam_t:ty, $($fields:ident),*) => {
        pub struct $t(pub $glam_t);
        impl<'lua> ToLua<'lua> for $t {
            fn to_lua(self, lua: &'lua Lua) -> mlua::Result<mlua::Value<'lua>> {
                let table =
                    lua.create_table_from([$((stringify!($fields), self.0.$fields)),*])?;
                Ok(mlua::Value::Table(table))
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

#[derive(Clone)]
pub struct Path(pub std::path::PathBuf);
impl UserData for Path {}

impl UserData for HalfEdgeMesh {}

macro_rules! def_wrapper_enum {
    ($tname:ident, $($a:ident => $b:ident),*) => {
        #[allow(dead_code)]
        pub enum $tname {
            $($a($b)),*
        }
        impl<'lua> ToLua<'lua> for $tname {
            fn to_lua(self, lua: &'lua Lua) -> mlua::Result<mlua::Value<'lua>> {
                match self {
                    $(EngineValue::$a(x) => x.to_lua(lua)),*
                }
            }
        }
    };
}

def_wrapper_enum!(EngineValue,
    Scalar => f32,
    Vec2 => Vec2,
    Vec3 => Vec3,
    Vec4 => Vec4,
    Selection => SelectionExpression,
    String => String,
    Path => Path,
    Mesh => HalfEdgeMesh
);

pub fn load_lua_libraries(lua: &Lua) -> anyhow::Result<()> {
    macro_rules! execute {
        ($file:expr) => {
            lua.load(include_str!($file)).exec()?
        };
    }

    macro_rules! def_library {
        ($name:expr, $file:expr) => {
            let lib: mlua::Value = lua.load(include_str!($file)).call(())?;
            lua.globals().set($name, lib)?;
        };
    }

    // Libraries
    def_library!("Fennel", "fennel-1.0.0.lua");
    def_library!("Vec2", "vec2.lua");
    def_library!("Vec3", "vec3.lua");
    def_library!("Vec4", "vec4.lua");
    def_library!("NodeLibrary", "node_library.lua");

    // Execute init code
    execute!("blackjack_init.lua");
    execute!("core_nodes.lua");

    Ok(())
}

pub fn load_node_libraries(lua: &Lua) -> anyhow::Result<HashMap<String, NodeDefinition>> {
    let table = lua
        .globals()
        .get::<_, Table>("NodeLibrary")?
        .get::<_, Table>("nodes")?;
    NodeDefinition::load_nodes_from_table(table)
}

/// Given a fresh lua instance, adds all the functions from blackjack's Lua
/// stdlib to the VM.
pub fn load_host_libraries(lua: &Lua) -> anyhow::Result<()> {
    let globals = lua.globals();

    let primitives = lua.create_table()?;
    globals.set("Primitives", primitives.clone())?;

    let ops = lua.create_table()?;
    globals.set("Ops", ops.clone())?;

    let export = lua.create_table()?;
    globals.set("Export", export.clone())?;

    let blackjack = lua.create_table()?;
    globals.set("Blackjack", blackjack.clone())?;

    macro_rules! lua_fn {
        ($table:ident, $name:expr, |$($argname:ident : $typ:ty),*| -> $retval:ty { $($body:tt)* }) => {
            $table.set($name,
                #[allow(unused_parens)]
                #[allow(unused_variables)]
                lua.create_function(|lua, ($($argname),*) : ($($typ),*)| -> mlua::Result<$retval> {
                    $($body)*
                })?
            )?
        };
    }

    lua_fn!(primitives, "cube", |center: Vec3,
                                 size: Vec3|
     -> HalfEdgeMesh {
        Ok(crate::mesh::halfedge::primitives::Box::build(
            center.0, size.0,
        ))
    });

    lua_fn!(primitives, "quad", |center: Vec3,
                                 normal: Vec3,
                                 right: Vec3,
                                 size: Vec2|
     -> HalfEdgeMesh {
        Ok(crate::mesh::halfedge::primitives::Quad::build(
            center.0, normal.0, right.0, size.0,
        ))
    });

    lua_fn!(export, "chamfer", |vertices: SelectionExpression,
                                amount: f32,
                                mesh: AnyUserData|
     -> HalfEdgeMesh {
        let mut result = mesh.borrow::<HalfEdgeMesh>()?.clone();
        result.clear_debug();
        for v in result.resolve_vertex_selection_full(vertices) {
            crate::mesh::halfedge::edit_ops::chamfer_vertex(&mut result, v, amount)
                .map_lua_err()?;
        }
        Ok(result)
    });

    lua_fn!(export, "bevel", |edges: SelectionExpression,
                              amount: f32,
                              mesh: AnyUserData|
     -> HalfEdgeMesh {
        let mut result = mesh.borrow::<HalfEdgeMesh>()?.clone();
        let edges = result.resolve_halfedge_selection_full(edges);
        crate::mesh::halfedge::edit_ops::bevel_edges(&mut result, &edges, amount).map_lua_err()?;
        Ok(result)
    });

    lua_fn!(export, "extrude", |faces: SelectionExpression,
                                amount: f32,
                                mesh: AnyUserData|
     -> HalfEdgeMesh {
        let mut result = mesh.borrow::<HalfEdgeMesh>()?.clone();
        let faces = result.resolve_face_selection_full(faces);
        crate::mesh::halfedge::edit_ops::extrude_faces(&mut result, &faces, amount)
            .map_lua_err()?;
        Ok(result)
    });

    lua_fn!(export, "merge", |a: AnyUserData,
                              b: AnyUserData|
     -> HalfEdgeMesh {
        let mut result = a.borrow::<HalfEdgeMesh>()?.clone();
        let b = b.borrow::<HalfEdgeMesh>()?;
        result.merge_with(&b);
        Ok(result)
    });

    lua_fn!(export, "wavefront_obj", |mesh: AnyUserData,
                                      path: Path|
     -> () {
        let mesh = mesh.borrow::<HalfEdgeMesh>()?;
        mesh.to_wavefront_obj(path.0).map_lua_err()?;
        Ok(())
    });

    lua_fn!(
        blackjack,
        "selection",
        |expr: mlua::String| -> SelectionExpression {
            SelectionExpression::parse(expr.to_str()?).map_lua_err()
        }
    );

    Ok(())
}

pub fn init_lua() -> anyhow::Result<LuaRuntime> {
    let lua = Lua::new();
    load_host_libraries(&lua)?;
    load_lua_libraries(&lua)?;
    let node_definitions = load_node_libraries(&lua)?;
    Ok(LuaRuntime {
        lua,
        node_definitions,
    })
}
