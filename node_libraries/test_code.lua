local function mesh(name) return {name = name, type = "mesh"} end

local perlin = Blackjack.perlin()

local test_channel_nodes = {
    AddNoiseTest = {
        label = "Add noise (Test)",
        op = function(inputs)
            local m = inputs.mesh:clone()

            local noise_ch = m:ensure_channel(Types.VertexId, Types.Vec3, "noise")
            local position_ch = m:get_channel(Types.VertexId, Types.Vec3, "position")

            for i,pos in ipairs(position_ch) do
                local noise_pos = pos * (1.0 / 0.323198);
                local noise = perlin:get_3d(noise_pos.x, noise_pos.y, noise_pos.z)
                noise_ch[i] = pos + vector(noise, noise, noise) * 0.025
            end

            m:set_channel(Types.VertexId, Types.Vec3, "position", noise_ch)

            return {out_mesh = m}
        end,
        inputs = {mesh("mesh")},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"

    },
}

NodeLibrary:addNodes(test_channel_nodes)