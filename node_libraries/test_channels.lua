local function mesh(name) return {name = name, type = "mesh"} end
local perlin = Blackjack.perlin()

local test_channel_nodes = {
    CreateRandom = {
        label = "Random channel",
        op = function(inputs)
            local fun = function(pos)
                local noise = perlin:get_3d(pos * (1.0 / 0.623198))
                return pos + Vec3(noise, noise, noise) * 0.025
            end
            local out = Ops.combine_channels(inputs.in_mesh, Types.VertexId,
                                             Types.Vec3, "position", Types.Vec3,
                                             "random", fun);
            return {out_mesh = out}
        end,
        inputs = {mesh("in_mesh")},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    },
    UpdatePosition = {
        label = "Displacement",
        op = function(inputs)
            local fun = function(pos)
                return pos
            end
            local out = Ops.combine_channels(inputs.in_mesh, Types.VertexId,
                                             Types.Vec3, "random", Types.Vec3,
                                             "position", fun);
            return {out_mesh = out}
        end,
        inputs = {mesh("in_mesh")},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    }
}

NodeLibrary:addNodes(test_channel_nodes)
