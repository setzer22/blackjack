-- Copyright (C) 2022 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

NodeLibrary = {
    nodes = {}
}

function NodeLibrary:addNodes(nodes)
    assert(type(nodes) == "table")

    for k, v in pairs(nodes) do
        if self.nodes[k] then
            print("[Engine] Redefinition for node "..k)
        else
            print("[Engine] Loading new node definition for "..k)
        end
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

function NodeLibrary:callNode(node_name, args)
    return self.nodes[node_name].op(args)
end

return NodeLibrary