// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

pub fn load(lua: &Lua) -> anyhow::Result<()> {
    let globals = lua.globals();
    let primitives = lua.create_table()?;
    globals.set("Primitives", primitives.clone())?;

    lua_fn!(lua, primitives, "cube", |center: LVec3,
                                      size: LVec3|
     -> HalfEdgeMesh {
        Ok(crate::mesh::halfedge::primitives::Box::build(
            center.0, size.0,
        ))
    });

    lua_fn!(lua, primitives, "quad", |center: LVec3,
                                      normal: LVec3,
                                      right: LVec3,
                                      size: LVec3|
     -> HalfEdgeMesh {
        Ok(crate::mesh::halfedge::primitives::Quad::build(
            center.0,
            normal.0,
            right.0,
            size.0.truncate(),
        ))
    });

    lua_fn!(lua, primitives, "circle", |center: LVec3,
                                        radius: f32,
                                        num_vertices: f32|
     -> HalfEdgeMesh {
        Ok(crate::mesh::halfedge::primitives::Circle::build_open(
            center.0,
            radius,
            num_vertices as usize,
        ))
    });

    lua_fn!(lua, primitives, "uv_sphere", |center: LVec3,
                                           radius: f32,
                                           segments: u32,
                                           rings: u32|
     -> HalfEdgeMesh {
        Ok(crate::mesh::halfedge::primitives::UVSphere::build(
            center.0, segments, rings, radius,
        ))
    });

    lua_fn!(lua, primitives, "line", |start: LVec3,
                                      end: LVec3,
                                      segments: u32|
     -> HalfEdgeMesh {
        Ok(crate::mesh::halfedge::primitives::Line::build_straight_line(start.0, end.0, segments))
    });


    lua_fn!(lua, primitives, "line_from_points", |points: Vec<LVec3>|
     -> HalfEdgeMesh {
        Ok(crate::mesh::halfedge::primitives::Line::build_from_points(LVec3::cast_vector(points)))
    });

    Ok(())
}
