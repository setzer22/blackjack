--
-- Copyright (C) 2022 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local P = require("params")
local NodeLibrary = require("node_library")
local V = require("vector_math")

NodeLibrary:addNodes({
    Playground = {
        label = "Playground",
        op = function(inputs)
            local mesh = HalfEdgeMesh.new()
            local cube = Primitives.cube(vector(0, 0, 0), vector(1, 1, 1))

            Ops.merge(mesh, cube)

            local ch = mesh:get_shared_channel(Types.VERTEX_ID, Types.VEC3, "position")
            for v in mesh:iter_vertices() do
                print(ch[v])
            end

            return { out_mesh = mesh }
        end,
        inputs = {},
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
})
