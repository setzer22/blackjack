-- Copyright (C) 2022 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local P = require("params")
local NodeLibrary = require("node_library")

NodeLibrary:addNodes({
    ReadCustomFile = {
        label = "Read custom file",
        op = function(inputs)
            local contents = Io.read_to_string(inputs.path)
            print(contents)
            return {
              out_mesh = Primitives.cube(vector(0,0,0), vector(1,1,1)),
              result = contents
            }
        end,
        inputs = {
            P.file("path", "open"),
        },
        outputs = {
            P.strparam("result"),
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
})
