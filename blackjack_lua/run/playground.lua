--
-- Copyright (C) 2022 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local P = require("params")
local V = require("vector_math")

NodeLibrary:addNodes({
    Playground = {
        label = "Playground",
        op = function(inputs)
            print(getmetatable(inputs.m1))
            local p2 = inputs.m1:clone()

            return {
                out_mesh = p2,
            }
        end,
        inputs = {
            P.mesh("m1"),
            P.mesh("m2"),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
})
