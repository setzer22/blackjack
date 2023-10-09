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
                return { out_mesh = Primitives.line_from_points(points) }
	    end

	    local angle_delta = max_angle / num_steps
	    local delta_y = inputs.size.y * inputs.turns / num_steps
	    local direction = inputs.direction == "Clockwise" and -1 or 1
	    local start_angle = math.pi * inputs.start_angle / 180
	    for i = 0, num_steps do
	    	local angle = direction * (start_angle + i * angle_delta)
	        local x = inputs.pos.x + inputs.size.x * math.cos(angle)
	        local z = inputs.pos.z + inputs.size.z * math.sin(angle)
	        local y = inputs.pos.y + i * delta_y
		table.insert(points, vector(x, y, z))
	    end
            return { out_mesh = Primitives.line_from_points(points) }
        end,
        inputs = {
            P.v3("pos", vector(0, 0, 0)),
            P.v3("size", vector(1, 1, 1)),
            P.scalar("start_angle", { default = 0, min = 0, soft_max = 360 }),
            P.scalar("turns", { default = 1, min = 0, soft_max = 10 }),
            P.scalar_int("segments", { default = 36, min = 1, soft_max = 360 }),
	    P.enum("direction", { "Clockwise", "Counter-Clockwise"}, 0),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
})
