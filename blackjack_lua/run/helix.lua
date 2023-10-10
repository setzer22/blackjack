local P = require("params")
local NodeLibrary = require("node_library")

NodeLibrary:addNodes({
    Helix = {
        label = "Helix",
        op = function(inputs)
            local points = {}
            -- Generate the points
            local max_angle = inputs.turns * 2.0 * math.pi
            local num_steps = math.ceil(inputs.turns * inputs.segments)

            if num_steps < 1 then
                return {
                    out_mesh = Primitives.line_from_points(points)
                }
            end

            local normals = {}
            local tangents = {}
            local angle_delta = max_angle / num_steps
            local delta_y = inputs.size.y * inputs.turns / num_steps
            local direction = inputs.direction == "Clockwise" and -1 or 1
            local start_angle = math.pi * inputs.start_angle / 180
            for i = 0, num_steps do
                local angle = direction * (start_angle + i * angle_delta)
                local cos_angle = math.cos(angle)
                local sin_angle = math.sin(angle)
                local x = inputs.pos.x + inputs.size.x * cos_angle
                local z = inputs.pos.z + inputs.size.z * sin_angle
                local y = inputs.pos.y + i * delta_y -- y is "up"
                table.insert(points, vector(x, y, z))
                local tx = -direction * sin_angle
                local tz = direction * cos_angle
                local ty = 0.0
                table.insert(tangents, vector(tx, ty, tz))
                -- local next_angle = direction * (start_angle + (i + 1) * angle_delta)
                -- local nx = inputs.size.x * (math.cos(next_angle) - cos_angle)
                -- local nz = inputs.size.z * (math.sin(next_angle) - sin_angle)
                -- local ny = delta_y
                local nx = 0.0
                local nz = 0.0
                local ny = 1.0
                table.insert(normals, vector(nx, ny, nz))
            end
            return {
                out_mesh = Primitives.line_with_normals(points, normals, tangents, num_steps)
            }
        end,
        inputs = {P.v3("pos", vector(0, 0, 0)), P.v3("size", vector(1, 1, 1)), P.scalar("start_angle", {
            default = 0,
            min = 0,
            soft_max = 360
        }), P.scalar("turns", {
            default = 1,
            min = 0,
            soft_max = 10
        }), P.scalar_int("segments", {
            default = 36,
            min = 1,
            soft_max = 360
        }), P.enum("direction", {"Clockwise", "Counter-Clockwise"}, 0)},
        outputs = {P.mesh("out_mesh")},
        returns = "out_mesh"
    }
})
