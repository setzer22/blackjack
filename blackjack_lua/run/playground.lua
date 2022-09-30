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
            local h = inputs.height
            local h2 = inputs.height2
            local c1 = Primitives.circle(vector(0, 0, 0), 1.0, 3)
            Ops.make_group(c1, Types.HalfEdgeId, Blackjack.selection("*"), "c1")
            local c2 = Primitives.circle(vector(0, h2, 0), 1.0, 3)
            Ops.make_group(c2, Types.HalfEdgeId, Blackjack.selection("*"), "c2")
            local c3 = Primitives.circle(vector(0, h + h2, 0), 1.0, 3)
            Ops.make_group(c3, Types.HalfEdgeId, Blackjack.selection("*"), "c3")
            local c4 = Primitives.circle(vector(0, h + 2 * h2, 0), 1.0, 3)
            Ops.make_group(c4, Types.HalfEdgeId, Blackjack.selection("*"), "c4")

            local mesh = Blackjack.mesh()
            Ops.merge(mesh, c1)
            Ops.merge(mesh, c2)
            Ops.merge(mesh, c3)
            Ops.merge(mesh, c4)

            Ops.bridge_chains(mesh, Blackjack.selection("@c1"), Blackjack.selection("@c2"), 0)
            Ops.bridge_chains(mesh, Blackjack.selection("@c3"), Blackjack.selection("@c4"), 0)
            Ops.bridge_chains(mesh, Blackjack.selection("@c2"), Blackjack.selection("@c3"), 0)

            return {
                out_mesh = mesh,
            }
        end,
        inputs = {
            P.scalar("height", { default = 1.0 }),
            P.scalar("height2", { default = 1.0 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
})
