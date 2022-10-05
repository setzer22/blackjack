// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.


use super::*;

pub fn load(lua: &Lua) -> anyhow::Result<()> {
    let globals = lua.globals();
    let ops = lua.create_table()?;
    globals.set("Ops", ops.clone())?;

    // WIP: Add a way to define lua constants like this:
    //
    // #[lua(under = "Types")]
    // const VertexId: ChannelKeyType = ChannelKeyType::VertexId;
    //
    // WIP: Also add a #[lua_extra] that receives methods and fields directly.

    let types = lua.create_table()?;
    types.set("VertexId", ChannelKeyType::VertexId)?;
    types.set("FaceId", ChannelKeyType::FaceId)?;
    types.set("HalfEdgeId", ChannelKeyType::HalfEdgeId)?;
    types.set("Vec3", ChannelValueType::Vec3)?;
    types.set("f32", ChannelValueType::f32)?;
    types.set("bool", ChannelValueType::bool)?;
    globals.set("Types", types)?;

    Ok(())
}
