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
            math.randomseed(12391823)
            local mesh = Blackjack.mesh()
            for i = 0, 100 do
                local x = 0
                local y = 0
                repeat
                    x = 2.0 * math.random() - 1.0
                    y = 2.0 * math.random() - 1.0
                until x * x + y * y <= 1.0

                local r = math.tan(inputs.angle)
                local v = V.normalize(vector(r * x, r * y, 1))

                local dir = V.normalize(inputs.dir)
                local rot_axis = V.cross(dir, vector(0, 0, 1))
                local rot_angle = math.acos(V.dot(vector(0, 0, 1), dir))

                local final
                if rot_angle > 0.05 then
                    final = V.rotate_around_axis(v, rot_axis, rot_angle)
                else
                    final = v
                end

                mesh:add_edge(vector(0, 0, 0), final)
            end
            return {
                out_mesh = mesh,
            }
        end,
        inputs = {
            P.scalar("angle", { default = 0.0, min = 0.0, max = 3.141592 }),
            P.v3("dir", vector(0, 0, 1)),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
})
