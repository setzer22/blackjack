local P = require("params")
local NodeLibrary = require("node_library")
local V = require("vector_math")

NodeLibrary:addNodes(
    {
        Helix = {
            label = "Helix",
            op = function(inputs)
                local points = {}
                local max_angle = inputs.turns * 2.0 * math.pi
                local total_segments = math.ceil(inputs.turns * inputs.segments)

                if total_segments < 1 then
                    return {
                        out_mesh = Primitives.line_from_points(points)
                    }
                end

                local normals = {}
                local tangents = {}
                local angle_delta = max_angle / total_segments
                local delta_y = inputs.size.y * inputs.turns / total_segments
                local direction = inputs.direction == "Clockwise" and -1 or 1
                local start_angle = math.pi * inputs.start_angle / 180
                for i = 0, total_segments do
                    local angle = direction * (start_angle + i * angle_delta)
                    local cos_angle = math.cos(angle)
                    local sin_angle = math.sin(angle)
                    local x = inputs.pos.x + inputs.size.x * cos_angle
                    local z = inputs.pos.z + inputs.size.z * sin_angle
                    local y = inputs.pos.y + i * delta_y -- y is "up"
                    local point = vector(x, y, z)
                    table.insert(points, point)
                    local tx = -direction * sin_angle
                    local tz = direction * cos_angle
                    local ty = 0.0
                    local tangent = V.normalize(vector(tx, ty, tz))
                    table.insert(tangents, tangent)
                    local nx = cos_angle
                    local nz = sin_angle
                    local ny = 0.0
                    local normal = V.normalize(V.cross(vector(nx, ny, nz), tangent))
                    table.insert(normals, normal)
                end
                return {
                    out_mesh = Primitives.line_with_normals(points, normals, tangents, total_segments)
                }
            end,
            inputs = {
                P.v3("pos", vector(0, 0, 0)),
                P.v3("size", vector(1, 1, 1)),
                P.scalar("start_angle", {default = 0, soft_max = 360}),
                P.scalar("turns", {default = 1, min = 0, soft_max = 10}),
                P.scalar_int("segments", {default = 36, min = 1, soft_max = 360}),
                P.enum("direction", {"Clockwise", "Counter-Clockwise"}, 0)
            },
            outputs = {P.mesh("out_mesh")},
            returns = "out_mesh"
        }
    }
)
