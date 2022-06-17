local function mesh(name) return {name = name, type = "mesh"} end
local function v3(name, default)
    return {name = name, default = default, type = "vec3"}
end
local function scalar(name, default, min, max)
    return {
        name = name,
        default = default,
        min = min,
        max = max,
        type = "scalar"
    }
end

local perlin = Blackjack.perlin()

local function normalize(v)
    local len = math.sqrt(v.x * v.x + v.y * v.y + v.z * v.z)
    return vector(v.x / len, v.y / len, v.z / len)
end

local test_channel_nodes = {
    AddNoiseTest = {
        label = "Add noise (Test)",
        op = function(inputs)
            local m = inputs.mesh:clone()

            local noise_ch = m:ensure_channel(Types.VertexId, Types.Vec3,
                                              "noise")
            local position_ch = m:get_channel(Types.VertexId, Types.Vec3,
                                              "position")

            for i, pos in ipairs(position_ch) do
                local noise_pos = pos * (1.0 / 0.323198);
                local noise = perlin:get_3d(noise_pos.x, noise_pos.y,
                                            noise_pos.z)
                noise_ch[i] = pos + vector(noise, noise, noise) * 0.1
            end

            m:set_channel(Types.VertexId, Types.Vec3, "position", noise_ch)

            return {out_mesh = m}
        end,
        inputs = {mesh("mesh")},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    },
    CopyToPoints = {
        label = "Copy to points",
        op = function(inputs)
            local points = inputs.points:get_channel(Types.VertexId, Types.Vec3,
                                                     "position")
            local acc = Blackjack.mesh()
            for i, pos in ipairs(points) do
                local new_mesh = Ops.translate(inputs.mesh:clone(), pos)
                Ops.merge(acc, new_mesh)
            end
            return {out_mesh = acc}
        end,
        inputs = {mesh("points"), mesh("mesh")},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    },
    CircleNoise = {
        label = "Circle Noise",
        op = function(inputs)
            local m = Primitives.circle(vector(0,0,0), 1.0, 12.0)
            local position_ch = m:get_channel(Types.VertexId, Types.Vec3, "position")
            for i, pos in ipairs(position_ch) do
                local noise_pos = pos * inputs.noise_scale + vector(inputs.seed, inputs.seed, inputs.seed);
                local noise = perlin:get_3d(noise_pos.x, noise_pos.y,
                                            noise_pos.z)
                local dir = normalize(pos)
                position_ch[i] = position_ch[i] + dir * noise * inputs.strength
            end
            m:set_channel(Types.VertexId, Types.Vec3, "position", position_ch)
            return {out_mesh = m}
        end,
        inputs = {scalar("strength", 0.1, 0.0, 1.0), scalar("noise_scale", 0.1, 0.01, 1.0), scalar("seed", 0.0, 0.0, 100.0)},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    },
    Capsule = {
        label = "Capsule",
        op = function (inputs)
            local m = Blackjack.mesh()
            local r = inputs.radius
            local rings = inputs.rings
            for ring=0,rings do
                local height = r * (ring / rings)
                local inner_radius = math.sqrt(r*r - height * height)

                local circle = Primitives.circle(vector(0,height,0), inner_radius, 12.0) 
                Ops.make_group(circle, Types.HalfEdgeId, Blackjack.selection("*"), "ring"..ring)
                Ops.merge(m, circle)
            end

            for ring=0,rings-1 do
                Ops.bridge_loops(m, Blackjack.selection("@ring"..ring), Blackjack.selection("@ring"..ring+1), 1)
            end

            return { out_mesh = m }
        end,
        inputs = {scalar("radius", 1.0, 0.0, 10.0), scalar("rings", 5.0, 1.0, 10.0)},
        outputs = {mesh("out_mesh")},
    }
}

NodeLibrary:addNodes(test_channel_nodes)
