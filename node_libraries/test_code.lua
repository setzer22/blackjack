local function mesh(name) return {name = name, type = "mesh"} end
local function v3(name, default)
    return {name = name, default = default, type = "vec3"}
end
local function selection(name) return {name = name, type = "selection"} end
local function scalar(name, default, min, max)
    return {
        name = name,
        default = default,
        min = min,
        max = max,
        type = "scalar"
    }
end
local function strparam(name, default, multiline)
    return {name = name, default = default, type = "string", multiline = multiline}
end

local perlin = Blackjack.perlin()

local function normalize(v: Vec3)
    local len = math.sqrt(v.x * v.x + v.y * v.y + v.z * v.z)
    return vector(v.x / len, v.y / len, v.z / len)
end

local function circle_noise(pos, radius, seed, strength, noise_scale, points)
    local m = Primitives.circle(pos, radius, points)
    local position_ch = m:get_channel(Types.VertexId, Types.Vec3, "position")
    for i, pos in ipairs(position_ch) do
        local noise_pos = pos * noise_scale + vector(seed, seed, seed);
        local noise = perlin:get_3d(noise_pos.x, noise_pos.y, noise_pos.z)
        local dir = normalize(pos)
        position_ch[i] = position_ch[i] + dir * noise * strength
    end
    m:set_channel(Types.VertexId, Types.Vec3, "position", position_ch)
    return m
end

local function parse_l_system_def(input: string): LSystemDef
    local axiom : string = ""
    local rules = {}
    for line in input:gmatch("[^\r\n]+") do
        if axiom == "" then
            local m = line:match("(%a+)")
            if m ~= nil then 
                axiom = m 
            else 
                print("Invalid axiom definition"..line)
                axiom = "NONE"
            end
        else
            local lhs : string?, rhs : string? = line:match("(%a+)%s+%->%s+([%a%[%]%+%-]+)")
            if lhs ~= nil and rhs ~= nil then
                table.insert(rules, { lhs = lhs, rhs = rhs })
            else
                print("Error parsing line of L-System: "..line)
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
    rules: {Rule},
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

local function l_system_substitution(sentence: string, l_system: LSystemDef) : string
    local new_sentence = ""
    for c in sentence:gmatch(".") do
        local some_matched = false
        for _,rule in l_system.rules do
            if c == rule.lhs then
                new_sentence = new_sentence..rule.rhs
                some_matched = true
            end
        end
        if not some_matched then
            new_sentence = new_sentence..c
        end
    end
    return new_sentence
end

local function l_system_iterate(l_system: LSystemDef, iterations: number) : string
    local sentence = l_system.axiom
    for i = 1,iterations do
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

local function rotate_vec(v: Vec3, angle: number) : Vec3
    local x = math.cos(angle) * v.x - math.sin(angle) * v.y
    local y = math.sin(angle) * v.x + math.cos(angle) * v.y
    return vector(x, y, v.z)
end

local PI = 3.1415926535

local function l_system_interpreter(sentence: string, params: LSystemParams) : {Edge}
    local turtle_stack : {Turtle} = {mk_turtle(vector(0,1,0), vector(0,0,0), 1.0)}
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
                final = false -- set later by the ] branch
            })
            turtle.position = end_point
            turtle.distance += 1
            turtle.did_draw = true
        end
    end
    return edges
end

local function make_l_system_mesh(edges: {Edge}) : any
    local mesh = Blackjack.mesh()
    local edge_distances = mesh:ensure_assoc_channel(Types.HalfEdgeId, Types.f32, "distance")
    local final_verts = mesh:ensure_assoc_channel(Types.VertexId, Types.bool, "final")
    for _, edge in edges do
        local h_start, h_end = mesh:add_edge(edge.start_point, edge.end_point)
        edge_distances[h_start] = edge.distance
        edge_distances[h_end] = -100 -- Mark as negative to ignore these edges
        final_verts[mesh:halfedge_vertex_id(h_end)] = edge.final
    end
    mesh:set_assoc_channel(Types.HalfEdgeId, Types.f32, "distance", edge_distances)
    mesh:set_assoc_channel(Types.VertexId, Types.bool, "final", final_verts)
    return mesh
end

local test_channel_nodes = {
    AddNoiseTest = {
        label = "Add noise (Test)",
        op = function(inputs)
            local m = inputs.mesh:clone()

            local noise_ch : {Vec3} = m:ensure_channel(Types.VertexId, Types.Vec3,
                                              "noise")
            local position_ch : {Vec3} = m:get_channel(Types.VertexId, Types.Vec3,
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
            local sizes = inputs.points:get_channel(Types.VertexId, Types.f32,
                                                     "size")
            local acc = Blackjack.mesh()
            for i, pos in ipairs(points) do
                local size = sizes[i]
                local new_mesh = inputs.mesh:clone()
                Ops.translate(new_mesh, pos, vector(0,0,0), vector(size, size, size))
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
            return {out_mesh = circle_noise(vector(0,0,0), 1.0, inputs.seed, inputs.strength, inputs.noise_scale, inputs.points)}
        end,
        inputs = {
            scalar("strength", 0.1, 0.0, 1.0), 
            scalar("noise_scale", 0.1, 0.01, 1.0),
            scalar("seed", 0.0, 0.0, 100.0),
            scalar("points", 8.0, 3.0, 16.0)
        },
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    },
    Capsule = {
        label = "Capsule",
        op = function (inputs)
            local m = Blackjack.mesh()
            local r : number = inputs.radius
            local height : number = inputs.height
            local rings = inputs.rings

            for ring=0,rings do
                local ring_height = r * (ring / rings)
                local inner_radius = math.sqrt(r*r - ring_height * ring_height)

                local circle = Primitives.circle(vector(0,ring_height + height / 2,0), inner_radius, 12.0) 
                Ops.make_group(circle, Types.HalfEdgeId, Blackjack.selection("*"), "ring"..ring)
                Ops.merge(m, circle)
            end

            for ring=0,rings do
                local ring_height = r * (ring / rings)
                local inner_radius = math.sqrt(r*r - ring_height * ring_height)

                local circle = Primitives.circle(vector(0,-ring_height - height / 2,0), inner_radius, 12.0)
                Ops.make_group(circle, Types.HalfEdgeId, Blackjack.selection("*"), "bot_ring"..ring)
                Ops.merge(m, circle)
            end

            for ring=0,rings-1 do
                Ops.bridge_loops(m, Blackjack.selection("@ring"..ring), Blackjack.selection("@ring"..ring+1), 1)
            end

            for ring=0,rings-1 do
                Ops.bridge_loops(m, Blackjack.selection("@bot_ring"..ring), Blackjack.selection("@bot_ring"..ring+1), 2)
            end

            Ops.bridge_loops(m, Blackjack.selection("@ring0"), Blackjack.selection("@bot_ring0"), 2)

            return { out_mesh = m }
        end,
        inputs = {
            scalar("radius", 1.0, 0.0, 10.0),
            scalar("rings", 5.0, 1.0, 10.0),
            scalar("height", 2.0, 0.0, 5.0),
        },
        outputs = {mesh("out_mesh")},
    },
    LSystem = {
        label = "L-System",
        op = function(inputs)
            local rule : string = inputs.rule
            local l_system_def = parse_l_system_def(rule);

            local params : LSystemParams = {
                forward_damp = inputs.forward_damp,
                angle_damp = inputs.angle_damp,
                initial_angle = inputs.initial_angle,
                initial_forward = inputs.initial_forward,
            }

            local sentence = l_system_iterate(l_system_def, math.floor(inputs.iterations))
            local edges = l_system_interpreter(sentence, params)
            local mesh = make_l_system_mesh(edges)

            return {out_mesh = mesh}
        end,
        inputs = {
            strparam("rule", "F+[F--F]", true), 
            scalar("iterations", 1, 1, 10),
            scalar("initial_forward", 1, 0, 5),
            scalar("forward_damp", 0.1, 0, 1),
            scalar("initial_angle", PI / 6, 0, 2 * PI),
            scalar("angle_damp", 0.1, 0, 1)
        } :: {any},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    },
    MakeTrunk = {
        label = "Make trunk",
        op = function(inputs)
            local edge_distances  = inputs.l_system:get_assoc_channel(Types.HalfEdgeId, Types.f32, "distance")
            local result = inputs.l_system:reduce_single_edges(
                Blackjack.mesh(),
                function (acc, h_id)
                    local scale_damp : number = inputs.scale_damp
                    if edge_distances[h_id] < 0 then
                        return acc
                    else
                        local src_pos, dst_pos = inputs.l_system:halfedge_endpoints(h_id)

                        local src_scale = 1.0 * scale_damp ^ edge_distances[h_id]
                        local dst_scale = 1.0 * scale_damp ^ (edge_distances[h_id] + 1)

                        local src_ring = inputs.ring:clone()
                        Ops.translate(src_ring, src_pos, vector(0,0,0), vector(src_scale,src_scale,src_scale))
                        Ops.make_group(src_ring, Types.HalfEdgeId, Blackjack.selection("*"), "src_ring")
                        
                        local dst_ring = inputs.ring:clone()
                        Ops.translate(dst_ring, dst_pos, vector(0,0,0), vector(dst_scale,dst_scale,dst_scale))
                        Ops.make_group(dst_ring, Types.HalfEdgeId, Blackjack.selection("*"), "dst_ring")

                        Ops.merge(src_ring, dst_ring)
                        Ops.bridge_loops(src_ring, Blackjack.selection("@src_ring"), Blackjack.selection("@dst_ring"), 1)

                        Ops.merge(acc, src_ring)
                        return acc
                    end
                end 
            )
            return { out_mesh = result }
        end,
        inputs = {
            mesh("l_system"),
            mesh("ring"),
            scalar("scale_damp", 0.95, 0.0, 1.0),
        } :: {any},
        outputs = { mesh("out_mesh") } :: {any},
        returns = "out_mesh",
    },
    MakeLeaves = {
        label = "Make leaves",
        op = function(inputs)
            local result = Blackjack.mesh()
            local final_verts = inputs.l_system:get_assoc_channel(Types.VertexId, Types.bool, "final")
            for v,is_final in final_verts do
                if is_final then
                    result:add_vertex(inputs.l_system:vertex_position(v))
                end
            end

            return { out_mesh = result }
        end,
        inputs = {
            mesh("l_system"),
        } :: {any},
        outputs = { mesh("out_mesh") } :: {any},
        returns = "out_mesh",
    },
    PointCloud = {
        label = "Point cloud",
        op = function(inputs)
            return { out_mesh = inputs.mesh:point_cloud(inputs.points) }
        end,
        inputs = {
            mesh("mesh"),
            selection("points")
        } :: {any},
        outputs = { mesh("out_mesh") } :: {any},
        returns = "out_mesh",
    },
    RandomizeSize = {
        label = "Randomize size",
        op = function(inputs)
            local mesh = inputs.mesh:clone()
            local size_ch = mesh:ensure_channel(Types.VertexId, Types.f32, "size")
            math.randomseed(inputs.seed)
            for i = 0,#size_ch do
                size_ch[i] = math.random() * inputs.scale
            end
            mesh:set_channel(Types.VertexId, Types.f32, "size", size_ch)
            return { out_mesh = mesh }
        end,
        inputs = {
            mesh("mesh"),
            scalar("scale", 1.0, 0.0, 2.0),
            scalar("seed", 0.0, 0.0, 100.0),
        } :: {any},
        outputs = { mesh("out_mesh") } :: {any},
        returns = "out_mesh",
    },
    PointCloudTessellate = {
        label = "Tessellate point cloud",
        op = function(inputs)
            return { out_mesh = Ops.point_cloud_tessellate(inputs.mesh) }
        end,
        inputs = {
            mesh("mesh"),
        } :: {any},
        outputs = { mesh("out_mesh") } :: {any},
        returns = "out_mesh",
    }
}

NodeLibrary:addNodes(test_channel_nodes)
