local function mesh(name) return {name = name, type = "mesh"} end
local perlin = Blackjack.perlin()

local test_channel_nodes = {
    CreateRandom = {
        label = "Random channel",
        op = function(inputs)
            local fun = function(pos)
                local noise = perlin:get_3d(pos * (1.0 / 0.323198))
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
            local fun = function(pos) return pos end
            local out = Ops.combine_channels(inputs.in_mesh, Types.VertexId,
                                             Types.Vec3, "random", Types.Vec3,
                                             "position", fun);
            return {out_mesh = out}
        end,
        inputs = {mesh("in_mesh")},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    },
    PullApi = {
        label = "Add noise (Pull)",
        op = function(inputs)
            local m = inputs.mesh:clone()
            local noise_ch = m:ensure_channel(Types.VertexId, Types.Vec3,
                                              "noise")
            local position_ch = m:get_channel(Types.VertexId, Types.Vec3,
                                              "position")
            for v in m:iter_vertices() do
                local pos = position_ch[v]
                local noise_pos = pos * (1.0 / 0.323198)
                local noise = perlin:get_3d(noise_pos.x, noise_pos.y, noise_pos.z)
                noise_ch[v] = Vec3(noise, noise, noise) * 0.025
            end
            return {out_mesh = m}
        end,
        inputs = {mesh("mesh")},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    },
    PullApi2 = {
        label = "Displacement (Pull)",
        op = function(inputs)
            local m = inputs.mesh:clone()
            local noise_ch = m:get_channel(Types.VertexId, Types.Vec3, "noise")
            local position_ch = m:get_channel(Types.VertexId, Types.Vec3,
                                              "position")
            for v in m:iter_vertices() do
                local pos = position_ch[v]
                local noise = noise_ch[v]
                position_ch[v] = pos + noise
            end
            return {out_mesh = m}
        end,
        inputs = {mesh("mesh")},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    },
}

NodeLibrary:addNodes(test_channel_nodes)
