-- Copyright (C) 2022 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local P = require("params")

local function load_function(code)
    local func, err = loadstring(code)
    if err ~= nil then
        error(err)
    end
    if typeof(func) ~= "function" then
        error("Code should be a single lua function")
    end
    return func
end

-- WIP: I need a priority queue library, but that means we have to implement
-- require. I think a good way to do require might be to split `node_libraries`
-- in two subfolders: One for the actual node libraries and the other for
-- additional user libraries (basically, things they can require)
--
-- The way this would work is that require('foo') looks for foo.lua in
-- lua/libraries, while node definitions are inside lua/node_libraries and
-- executed on any file change.
--
-- Since users also may want to get hot-reloading for libraries, once we detect
-- changes we will clear the _LOADED table and any requires from
-- lua/node_libraries will transitively get reloaded again.
--
-- WIP 2: I have the LuaFileIo trait and a StdLuaFileIo implementation, now I
-- need to replace the old Box<dyn Fn() -> NodeLibraries> with this trait
--
-- - [ ] Replace the box dyn fn field
-- - [ ] Change the load_node_libraries_with_std function for a platform-independent
--       function that uses the trait for the platform-specific bits
-- - [ ] Use the new parts of the function during `require`, this may
--       require storing the dyn LuaFileIo inside an Arc, so it can be moved
--       inside the closure
-- - [ ] Implement LuaFileIo for the godot platform


local test_channel_nodes = {
    ProceduralRoad = {
        label = "Procedural road",
        op = function(inputs)
            local noise = load_function(inputs.noise_fn)
            local noise_fn = function(pos)
                local scale = 0.05 -- TODO @Hardcoded @Heightmap
                local j = pos.x / scale
                local i = pos.z / scale
                return vector(pos.x, noise(i, j), pos.z)
            end
            local mesh = Primitives.cube(noise_fn(inputs.src), vector(0.1, 0.1, 0.1))
            Ops.merge(mesh, Primitives.cube(noise_fn(inputs.dst), vector(0.1, 0.1, 0.1)))

            local src = inputs.src
            local dst = inputs.dst
            local resolution = 30.0

            for i = 1, resolution do
                local p = noise_fn(src + (dst - src) * ((i - 1) / resolution))
                local p2 = noise_fn(src + (dst - src) * (i / resolution))
                mesh:add_edge(p, p2)
            end

            return { out_mesh = mesh }
        end,
        inputs = {
            P.lua_str("noise_fn"),
            P.v3("src", vector(0.1, 0.1, 0.1)),
            P.v3("dst", vector(0.9, 0.9, 0.9)),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
}

NodeLibrary:addNodes(test_channel_nodes)
