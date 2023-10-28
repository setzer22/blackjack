-- Copyright (C) 2023 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local NodeLibrary = {
    nodes = {}
}

function NodeLibrary:addNodes(nodes)
    assert(type(nodes) == "table")

    for k, v in pairs(nodes) do
	-- mlua does not provide io.stderr, and this module can't be
	-- used to directly generate code if it prints to stdout, so
	-- the following needs to be commented out until mlua supports io.stderr:
        -- if self.nodes[k] then
        --     io.stderr:write("[Engine] Redefinition for node "..k.."\n")
        -- else
        --     io.stderr:write("[Engine] Loading new node definition for "..k.."\n")
        -- end
        self.nodes[k] = v
    end
end

function NodeLibrary:listNodes()
    local nodes = {}
    for k, _ in pairs(self.nodes) do
        table.insert(nodes, k)
    end
    return nodes
end

function NodeLibrary:getNode(node_name)
    return self.nodes[node_name]
end

return NodeLibrary
