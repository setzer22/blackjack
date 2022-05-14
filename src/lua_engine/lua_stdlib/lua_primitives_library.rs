use super::*;

pub fn load(lua: &Lua) -> anyhow::Result<()> {
    let globals = lua.globals();
    let primitives = lua.create_table()?;
    globals.set("Primitives", primitives.clone())?;

    lua_fn!(lua, primitives, "cube", |center: Vec3,
                                      size: Vec3|
     -> HalfEdgeMesh {
        Ok(crate::mesh::halfedge::primitives::Box::build(
            center.0, size.0,
        ))
    });

    lua_fn!(lua, primitives, "quad", |center: Vec3,
                                      normal: Vec3,
                                      right: Vec3,
                                      size: Vec3|
     -> HalfEdgeMesh {
        Ok(crate::mesh::halfedge::primitives::Quad::build(
            center.0,
            normal.0,
            right.0,
            size.0.truncate(),
        ))
    });

    lua_fn!(lua, primitives, "circle", |center: Vec3,
                                        radius: f32,
                                        num_vertices: f32|
     -> HalfEdgeMesh {
        Ok(crate::mesh::halfedge::primitives::Circle::build(
            center.0,
            radius,
            num_vertices as usize,
        ))
    });

    Ok(())
}
