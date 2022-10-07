-- Copyright (C) 2022 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local P = require("params")
local NodeLibrary = require("node_library")

local perlin = PerlinNoise.new()

local function normalize(v: Vec3)
    local len = math.sqrt(v.x * v.x + v.y * v.y + v.z * v.z)
    return vector(v.x / len, v.y / len, v.z / len)
end

local function load_function(code)
    local func, err = loadstring(code)
    if err ~= nil then
        error(err)
    end
    if typeof(func) ~= "function" then
        error("Code should be a single lua function")
    end
    return func
end

local function circle_noise(pos, radius, seed, strength, noise_scale, points)
    local m = Primitives.circle(pos, radius, points)
    local position_ch = m:get_channel(Types.VERTEX_ID, Types.VEC3, "position")
    for i, pos in ipairs(position_ch) do
        local noise_pos = pos * noise_scale + vector(seed, seed, seed)
        local noise = perlin:get_3d(noise_pos.x, noise_pos.y, noise_pos.z)
        local dir = normalize(pos)
        position_ch[i] = position_ch[i] + dir * noise * strength
    end
    m:set_channel(Types.VERTEX_ID, Types.VEC3, "position", position_ch)
    return m
end

local function parse_l_system_def(input: string): LSystemDef
    local axiom: string = ""
    local rules = {}
    for line in input:gmatch("[^\r\n]+") do
        if axiom == "" then
            local m = line:match("(%a+)")
            if m ~= nil then
                axiom = m
            else
                print("Invalid axiom definition" .. line)
                axiom = "NONE"
            end
        else
            local lhs: string?, rhs: string? = line:match("(%a+)%s+%->%s+([%a%[%]%+%-]+)")
            if lhs ~= nil and rhs ~= nil then
                table.insert(rules, { lhs = lhs, rhs = rhs })
            else
                print("Error parsing line of L-System: " .. line)
            end
        end
    end
    return { rules = rules, axiom = axiom }
end

type Rule = {
    lhs: string,
    rhs: string,
}
type LSystemDef = {
    rules: { Rule },
    axiom: string,
}

type Vec3 = any
type Turtle = {
    facing: Vec3,
    position: Vec3,
    distance: number,
    did_draw: boolean,
}
type Edge = {
    start_point: Vec3,
    end_point: Vec3,
    distance: number,
    final: boolean,
}
type LSystemParams = {
    forward_damp: number,
    angle_damp: number,
    initial_angle: number,
    initial_forward: number,
}

local function l_system_substitution(sentence: string, l_system: LSystemDef): string
    local new_sentence = ""
    for c in sentence:gmatch(".") do
        local some_matched = false
        for _, rule in l_system.rules do
            if c == rule.lhs then
                new_sentence = new_sentence .. rule.rhs
                some_matched = true
            end
        end
        if not some_matched then
            new_sentence = new_sentence .. c
        end
    end
    return new_sentence
end

local function l_system_iterate(l_system: LSystemDef, iterations: number): string
    local sentence = l_system.axiom
    for i = 1, iterations do
        sentence = l_system_substitution(sentence, l_system)
    end
    return sentence
end

local function mk_turtle(facing: Vec3, position: Vec3, distance: number): Turtle
    return {
        facing = facing,
        position = position,
        distance = distance,
        did_draw = false,
    }
end

local function rotate_vec(v: Vec3, angle: number): Vec3
    local x = math.cos(angle) * v.x - math.sin(angle) * v.y
    local y = math.sin(angle) * v.x + math.cos(angle) * v.y
    return vector(x, y, v.z)
end

local PI = 3.1415926535

local function l_system_interpreter(sentence: string, params: LSystemParams): { Edge }
    local turtle_stack: { Turtle } = { mk_turtle(vector(0, 1, 0), vector(0, 0, 0), 1.0) }
    local edges = {}

    for c in sentence:gmatch(".") do
        local turtle = turtle_stack[#turtle_stack]

        if c == "+" then
            local angle = params.initial_angle * params.angle_damp ^ turtle.distance
            turtle.facing = rotate_vec(turtle.facing, angle)
        elseif c == "[" then
            table.insert(turtle_stack, mk_turtle(turtle.facing, turtle.position, turtle.distance))
        elseif c == "]" then
            table.remove(turtle_stack, #turtle_stack)
            if turtle.did_draw then
                edges[#edges].final = true
            end
        elseif c == "-" then
            local angle = params.initial_angle * params.angle_damp ^ turtle.distance
            turtle.facing = rotate_vec(turtle.facing, -angle)
        elseif c == "F" then
            local forward = params.initial_forward * params.forward_damp ^ turtle.distance
            local end_point = turtle.position + turtle.facing * forward
            table.insert(edges, {
                start_point = turtle.position,
                end_point = end_point,
                distance = turtle.distance,
                final = false, -- set later by the ] branch
            })
            turtle.position = end_point
            turtle.distance += 1
            turtle.did_draw = true
        end
    end
    return edges
end

local function make_l_system_mesh(edges: { Edge }): any
    local mesh = HalfEdgeMesh.new()
    local edge_distances = mesh:ensure_assoc_channel(Types.HALFEDGE_ID, Types.F32, "distance")
    local final_verts = mesh:ensure_assoc_channel(Types.VERTEX_ID, Types.BOOL, "final")
    for _, edge in edges do
        local h_start, h_end = mesh:add_edge(edge.start_point, edge.end_point)
        edge_distances[h_start] = edge.distance
        edge_distances[h_end] = -100 -- Mark as negative to ignore these edges
        final_verts[mesh:halfedge_vertex_id(h_end)] = edge.final
    end
    mesh:set_assoc_channel(Types.HALFEDGE_ID, Types.F32, "distance", edge_distances)
    mesh:set_assoc_channel(Types.VERTEX_ID, Types.BOOL, "final", final_verts)
    return mesh
end

local test_channel_nodes = {
    AddNoiseTest = {
        label = "Add noise (Test)",
        op = function(inputs)
            local m = inputs.mesh:clone()

            local position_ch: { Vec3 } = m:get_channel(Types.VERTEX_ID, Types.VEC3, "position")
            local normal_ch: { Vec3 } = m:get_channel(Types.VERTEX_ID, Types.VEC3, "vertex_normal")

            for i, pos in ipairs(position_ch) do
                local noise_pos = pos * inputs.scale + inputs.offset
                local noise = perlin:get_3d(noise_pos.x, noise_pos.y, noise_pos.z)
                position_ch[i] = pos + normal_ch[i] * noise * inputs.strength
            end

            m:set_channel(Types.VERTEX_ID, Types.VEC3, "position", position_ch)

            return { out_mesh = m }
        end,
        inputs = {
            P.mesh("mesh"),
            P.scalar("scale", { default = 3.0, min = 0.0, max = 10.0 }),
            P.v3("offset", vector(0, 0, 0)),
            P.scalar("strength", { default = 0.1, min = 0.0, max = 1.0 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    CircleNoise = {
        label = "Circle Noise",
        op = function(inputs)
            return {
                out_mesh = circle_noise(
                    vector(0, 0, 0),
                    1.0,
                    inputs.seed,
                    inputs.strength,
                    inputs.noise_scale,
                    inputs.points
                ),
            }
        end,
        inputs = {
            P.scalar("strength", { default = 0.1, min = 0.0, max = 1.0 }),
            P.scalar("noise_scale", { default = 0.1, min = 0.01, max = 1.0 }),
            P.scalar("seed", { default = 0.0, min = 0.0, max = 100.0 }),
            P.scalar("points", { default = 8.0, min = 3.0, max = 16.0 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    Capsule = {
        label = "Capsule",
        op = function(inputs)
            local m = HalfEdgeMesh.new()
            local r: number = inputs.radius
            local height: number = inputs.height
            local rings = inputs.rings

            local selection = SelectionExpression.new

            for ring = 0, rings do
                local ring_height = r * (ring / rings)
                local inner_radius = math.sqrt(r * r - ring_height * ring_height)

                local circle = Primitives.circle(vector(0, ring_height + height / 2, 0), inner_radius, inputs.sections)
                Ops.make_group(circle, Types.HALFEDGE_ID, selection("*"), "ring" .. ring)
                Ops.merge(m, circle)
            end

            for ring = 0, rings do
                local ring_height = r * (ring / rings)
                local inner_radius = math.sqrt(r * r - ring_height * ring_height)

                local circle = Primitives.circle(vector(0, -ring_height - height / 2, 0), inner_radius, inputs.sections)
                Ops.make_group(circle, Types.HALFEDGE_ID, selection("*"), "bot_ring" .. ring)
                Ops.merge(m, circle)
            end

            for ring = 0, rings - 1 do
                Ops.bridge_chains(m, selection("@ring" .. ring), selection("@ring" .. ring + 1), 0)
            end

            for ring = 0, rings - 1 do
                Ops.bridge_chains(
                    m,
                    selection("@bot_ring" .. ring),
                    selection("@bot_ring" .. ring + 1),
                    1
                )
            end

            Ops.bridge_chains(m, selection("@ring0"), selection("@bot_ring0"), 1)

            return { out_mesh = m }
        end,
        inputs = {
            P.scalar("radius", { default = 1.0, min = 0.0, max = 10.0 }),
            P.scalar_int("rings", { default = 5, min = 1, soft_max = 10 }),
            P.scalar_int("sections", { default = 12, min = 1, soft_max = 32 }),
            P.scalar("height", { default = 2.0, min = 0.0, max = 5.0 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    LSystem = {
        label = "L-System",
        op = function(inputs)
            local rule: string = inputs.rule
            local l_system_def = parse_l_system_def(rule)

            local params: LSystemParams = {
                forward_damp = inputs.forward_damp,
                angle_damp = inputs.angle_damp,
                initial_angle = inputs.initial_angle,
                initial_forward = inputs.initial_forward,
            }

            local sentence = l_system_iterate(l_system_def, math.floor(inputs.iterations))
            local edges = l_system_interpreter(sentence, params)
            local mesh = make_l_system_mesh(edges)

            return { out_mesh = mesh }
        end,
        inputs = {
            P.strparam("rule", "F+[F--F]", true),
            P.scalar_int("iterations", { default = 1, min = 1, max = 10 }),
            P.scalar_int("initial_forward", { default = 1, min = 0, max = 5 }),
            P.scalar("forward_damp", { default = 0.1, min = 0, max = 1 }),
            P.scalar("initial_angle", { default = PI / 6, min = 0, max = 2 * PI }),
            P.scalar("angle_damp", { default = 0.1, min = 0, max = 1 }),
        } :: { any },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    MakeTrunk = {
        label = "Make trunk",
        op = function(inputs)
            local edge_distances = inputs.l_system:get_assoc_channel(Types.HALFEDGE_ID, Types.F32, "distance")
            local result = inputs.l_system:reduce_halfedges(HalfEdgeMesh.new(), function(acc, h_id)
                local scale_damp: number = inputs.scale_damp
                if edge_distances[h_id] < 0 then
                    return acc
                else
                    local src_pos, dst_pos = inputs.l_system:halfedge_endpoints(h_id)

                    local src_scale = 1.0 * scale_damp ^ edge_distances[h_id]
                    local dst_scale = 1.0 * scale_damp ^ (edge_distances[h_id] + 1)
                    local selection = SelectionExpression.new

                    local src_ring = inputs.ring:clone()
                    Ops.transform(src_ring, src_pos, vector(0, 0, 0), vector(src_scale, src_scale, src_scale))
                    Ops.make_group(src_ring, Types.HALFEDGE_ID, selection("*"), "src_ring")

                    local dst_ring = inputs.ring:clone()
                    Ops.transform(dst_ring, dst_pos, vector(0, 0, 0), vector(dst_scale, dst_scale, dst_scale))
                    Ops.make_group(dst_ring, Types.HALFEDGE_ID, selection("*"), "dst_ring")

                    Ops.merge(src_ring, dst_ring)
                    Ops.bridge_chains(src_ring, selection("@src_ring"), selection("@dst_ring"), 0)

                    Ops.merge(acc, src_ring)
                    return acc
                end
            end)
            return { out_mesh = result }
        end,
        inputs = {
            P.mesh("l_system"),
            P.mesh("ring"),
            P.scalar("scale_damp", { default = 0.95, min = 0.0, max = 1.0 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    MakeLeaves = {
        label = "Make leaves",
        op = function(inputs)
            local result = HalfEdgeMesh.new()
            local final_verts = inputs.l_system:get_assoc_channel(Types.VERTEX_ID, Types.BOOL, "final")
            for v, is_final in final_verts do
                if is_final then
                    result:add_vertex(inputs.l_system:vertex_position(v))
                end
            end

            return { out_mesh = result }
        end,
        inputs = {
            P.mesh("l_system"),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    PointCloud = {
        label = "Point cloud",
        op = function(inputs)
            return { out_mesh = inputs.mesh:point_cloud(inputs.points) }
        end,
        inputs = {
            P.mesh("mesh"),
            P.selection("points"),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    RandomizeSize = {
        label = "Randomize size",
        op = function(inputs)
            local mesh = inputs.mesh:clone()
            local size_ch = mesh:ensure_channel(Types.VERTEX_ID, Types.F32, "size")
            math.randomseed(inputs.seed)
            for i = 0, #size_ch do
                size_ch[i] = math.random() * inputs.scale
            end
            mesh:set_channel(Types.VERTEX_ID, Types.F32, "size", size_ch)
            return { out_mesh = mesh }
        end,
        inputs = {
            P.mesh("mesh"),
            P.scalar("scale", { default = 1.0, min = 0.0, max = 2.0 }),
            P.scalar("seed", { default = 0.0, min = 0.0, max = 100.0 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
}

NodeLibrary:addNodes(test_channel_nodes)
