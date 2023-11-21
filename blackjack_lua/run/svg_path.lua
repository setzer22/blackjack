local P = require("params")
local NodeLibrary = require("node_library")
local V = require("vector_math")

local signed_mod = function(a, b)
    if a > b then
        return a % b
    elseif a < -b then
        return -(-a % b)
    end
    return a
end

local rotate_around_axis = function(angle, x, y)
    return vector(
        math.cos(angle)*x - math.sin(angle)*y,
        math.sin(angle)*x + math.cos(angle)*y,
        0)
end

local parse_parameters = function(d)
    local params = {}
    while string.len(d) > 0 do
        local _, j, m = string.find(d, "^%s*([%d%.%-]+)%s*,*")
        if m ~= nil then
            table.insert(params, 0+m)  -- coerce m to a number.
            d = string.sub(d, j+1)
        else
            -- Should not reach here.
            print("programming error - parse_parameters")
            return params
        end
    end
    return params
end

local parse_path = function(d)
    local path_steps = {}
    while string.len(d) > 0 do
        local i, j = string.find(d, "^%s+")
        if i ~= nil then
            d = string.sub(d, j+1) -- strip leading whitespace
        end

        local _, j, m = string.find(d, "^(z)%s*")
        if m ~= nil then
            table.insert(path_steps, {C="z"})
            d = string.sub(d, j+1)
        else
            local _, j, m, n = string.find(d, "^([mlhvcsqtaMLHVCSQTA])([%d%.%-,%s]*)")
            if m ~= nil then
                local params = parse_parameters(n)
                table.insert(path_steps, {C=m, P=params})
                d = string.sub(d, j+1)
            else
                -- Should not reach here, as this is an unsupported SVG command.
                print("unsupported SVG path command:", d)
                return path_steps
            end
        end
    end
    return path_steps
end

local cmd_close_path = function(state)
    if #state.points > 0 then
        if state.mesh ~= nil then
            Ops.merge(state.mesh, Primitives.polygon(state.points))
        else
            state.mesh = Primitives.polygon(state.points)
        end
        state.points = {}
    end
end

local terminate_path = function(state)
    if #state.points > 0 then
        if state.mesh ~= nil then
            Ops.merge(state.mesh, Primitives.line_from_points(state.points))
        else
            state.mesh = Primitives.line_from_points(state.points)
        end
        state.points = {}
    end
end

local insert_current_pos = function(state)
    table.insert(state.points, (state.current_pos.x * state.right + state.current_pos.y * state.normal)+vector(0,0,state.pos.z))
end

local cmd_move_to_abs = function(state, params)
    terminate_path(state)
    state.current_pos = state.pos + vector(params[1], params[2], 0) * state.size
    insert_current_pos(state)
    return state
end

local cmd_move_to = function(state, params)
    terminate_path(state)
    state.current_pos = state.current_pos + vector(params[1], params[2], 0) * state.size
    insert_current_pos(state)
    return state
end

local cmd_line_to_abs = function(state, params)
    state.current_pos = state.pos + vector(params[1], params[2], 0) * state.size
    insert_current_pos(state)
    return state
end

local cmd_line_to = function(state, params)
    for i = 1, #params, 2 do
        state.current_pos = state.current_pos + vector(params[i], params[i+1], 0) * state.size
        insert_current_pos(state)
    end
    return state
end

local cmd_line_horizontal_abs = function(state, params)
    for i = 1, #params do
        state.current_pos = state.current_pos*vector(0,1,0) + vector(params[i],0,0) * state.size
        insert_current_pos(state)
    end
    return state
end

local cmd_line_horizontal = function(state, params)
    for i = 1, #params do
        state.current_pos = state.current_pos + vector(params[i],0,0) * state.size
        insert_current_pos(state)
    end
    return state
end

local cmd_line_vertical_abs = function(state, params)
    for i = 1, #params do
        state.current_pos = state.current_pos*vector(1,0,0) + vector(0, params[i], 0) * state.size
        insert_current_pos(state)
    end
    return state
end

local cmd_line_vertical = function(state, params)
    for i = 1, #params do
        state.current_pos = state.current_pos + vector(0, params[i], 0) * state.size
        insert_current_pos(state)
    end
    return state
end

local cubic_to = function(state, p0, p1, p2, p3)
    for i = 1, state.segments do
        local t = i / state.segments
        local t1 = 1.0 - t
        local f = t1 * t1 * t1
        state.current_pos = p0 * f
        f = 3.0 * t1 * t1 * t
        state.current_pos = state.current_pos + p1 * f
        f = 3.0 * t1 * t * t
        state.current_pos = state.current_pos + p2 * f
        f = t * t * t
        state.current_pos = state.current_pos + p3 * f
        insert_current_pos(state)
    end
    return state
end

local cmd_cubic_bezier_curve_abs = function(state, params)
    for i = 1, #params, 6 do
        local p0 = state.current_pos
        local p1 = state.pos + vector(params[i  ], params[i+1], 0) * state.size
        local p2 = state.pos + vector(params[i+2], params[i+3], 0) * state.size
        local ep = state.pos + vector(params[i+4], params[i+5], 0) * state.size
        state = cubic_to(state, p0, p1, p2, ep)
        state.last_p2 = p2
        state.last_p3 = ep
    end
    return state
end

local cmd_cubic_bezier_curve = function(state, params)
    for i = 1, #params, 6 do
        local p0 = state.current_pos
        local p1 = state.current_pos + vector(params[i  ], params[i+1], 0) * state.size
        local p2 = state.current_pos + vector(params[i+2], params[i+3], 0) * state.size
        local ep = state.current_pos + vector(params[i+4], params[i+5], 0) * state.size
        state = cubic_to(state, p0, p1, p2, ep)
        state.last_p2 = p2
        state.last_p3 = ep
    end
    return state
end

local cmd_smooth_cubic_bezier_curve_abs = function(state, params)
    for i = 1, #params, 4 do
        local p0 = state.current_pos
        local p1 = state.pos
        if state.last_cmd == "C" or state.last_cmd == "c" or state.last_cmd == "S" or state.last_cmd == "s" then
            p1 = state.current_pos + state.last_p3 - state.last_p2
        end
        local p2 = state.pos + vector(params[i  ], params[i+1], 0) * state.size
        local ep = state.pos + vector(params[i+2], params[i+3], 0) * state.size
        state = cubic_to(state, p0, p1, p2, ep)
        state.last_p2 = p2
        state.last_p3 = ep
    end
    return state
end

local cmd_smooth_cubic_bezier_curve = function(state, params)
    for i = 1, #params, 4 do
        local p0 = state.current_pos
        local p1 = state.current_pos
        if state.last_cmd == "C" or state.last_cmd == "c" or state.last_cmd == "S" or state.last_cmd == "s" then
            p1 = state.current_pos + state.last_p3 - state.last_p2
        end
        local p2 = state.current_pos + vector(params[i  ], params[i+1], 0) * state.size
        local ep = state.current_pos + vector(params[i+2], params[i+3], 0) * state.size
        state = cubic_to(state, p0, p1, p2, ep)
        state.last_p2 = p2
        state.last_p3 = ep
    end
    return state
end

local quadratic_to = function(state, p0, p1, p2)
    for i = 1, state.segments do
        local t = i / state.segments
        local t1 = 1.0 - t
        local f = t1 * t1
        state.current_pos = p0 * f
        f = 2.0 * t1 * t
        state.current_pos = state.current_pos + p1 * f
        f = t * t
        state.current_pos = state.current_pos + p2 * f
        insert_current_pos(state)
    end
    return state
end

local cmd_quadratic_bezier_curve_abs = function(state, params)
    for i = 1, #params, 4 do
        local p0 = state.current_pos
        local p1 = state.pos + vector(params[i  ], params[i+1], 0) * state.size
        local p2 = state.pos + vector(params[i+2], params[i+3], 0) * state.size
        state = quadratic_to(state, p0, p1, p2)
        state.last_p1 = p1
        state.last_p2 = p2
    end
    return state
end

local cmd_quadratic_bezier_curve = function(state, params)
    for i = 1, #params, 4 do
        local p0 = state.current_pos
        local p1 = state.current_pos + vector(params[i  ], params[i+1], 0) * state.size
        local p2 = state.current_pos + vector(params[i+2], params[i+3], 0) * state.size
        state = quadratic_to(state, p0, p1, p2)
        state.last_p1 = p1
        state.last_p2 = p2
    end
    return state
end

local cmd_smooth_quadratic_bezier_curve_abs = function(state, params)
    for i = 1, #params, 2 do
        local p0 = state.current_pos
        local p1 = state.current_pos
        if state.last_cmd == "Q" or state.last_cmd == "q" or state.last_cmd == "T" or state.last_cmd == "t" then
            p1 = state.current_pos + state.last_p2 - state.last_p1
        end
        local p2 = state.pos + vector(params[i  ], params[i+1], 0) * state.size
        state = quadratic_to(state, p0, p1, p2)
        state.last_p1 = p1
        state.last_p2 = p2
    end
    return state
end

local cmd_smooth_quadratic_bezier_curve = function(state, params)
    for i = 1, #params, 2 do
        local p0 = state.current_pos
        local p1 = state.current_pos
        if state.last_cmd == "Q" or state.last_cmd == "q" or state.last_cmd == "T" or state.last_cmd == "t" then
            p1 = state.current_pos + state.last_p2 - state.last_p1
        end
        local p2 = state.current_pos + vector(params[i  ], params[i+1], 0) * state.size
        state = quadratic_to(state, p0, p1, p2)
        state.last_p1 = p1
        state.last_p2 = p2
    end
    return state
end

local angle_between = function(v0, v1)
    local p =  v0.x*v1.x + v0.y*v1.y
    local n = math.sqrt((math.pow(v0.x, 2)+math.pow(v0.y, 2)) * (math.pow(v1.x, 2)+math.pow(v1.y, 2)))
    local sign = v0.x*v1.y - v0.y*v1.x < 0 and -1 or 1
    local angle = sign*math.acos(p/n)
    return angle
end

-- based on: https://ericeastwood.com/blog/curves-and-arcs-quadratic-cubic-elliptical-svg-implementations/
local elliptic_arc_to = function(state, rx, ry, angle, large_arc_flag, sweep_flag, p0, p1)
    if radius == vector(0,0,0) then  -- zero radius degrades to straight line
        state.current_pos = p1
        insert_current_pos(state)
        return
    end

    local d = 0.5*(p0 - p1)
    local tpx = math.cos(angle)*d.x + math.sin(angle)*d.y
    local tpy = -math.sin(angle)*d.x + math.cos(angle)*d.y
    local radii_check = math.pow(tpx,2)/math.pow(rx,2) + math.pow(tpy,2)/math.pow(ry,2)
    if radii_check > 1 then
        rx = math.sqrt(radii_check)*rx
        ry = math.sqrt(radii_check)*ry
    end

    local sn = math.pow(rx,2)*math.pow(ry,2) - math.pow(rx,2)*math.pow(tpy,2) - math.pow(ry,2)*math.pow(tpx,2)
    local srd = math.pow(rx,2)*math.pow(tpy,2) + math.pow(ry,2)*math.pow(tpx,2)
    local radicand = sn/srd
    if radicand < 0 then
        radicand = 0
    end

    local coef = large_arc_flag ~= sweep_flag and math.sqrt(radicand) or -math.sqrt(radicand)
    local tcx = coef*((rx*tpy)/ry)
    local tcy = coef*(-(ry*tpx)/rx)
    local tc = vector(tcx, tcy, 0)

    local center = rotate_around_axis(angle, tcx, tcy) + (0.5*(p0 + p1))

    local start_vector = vector((tpx - tcx)/rx, (tpy - tcy)/ry, 0)
    local start_angle = angle_between(vector(1,0,0), start_vector)

    local end_vector = vector((-tpx - tcx)/rx, (-tpy - tcy)/ry, 0)
    local sweep_angle = angle_between(start_vector, end_vector)

    if sweep_flag == 0 and sweep_angle > 0 then
        sweep_angle = sweep_angle - 2*math.pi
    elseif sweep_flag ~= 0 and sweep_angle < 0 then
        sweep_angle = sweep_angle + 2*math.pi
    end
    sweep_angle = signed_mod(sweep_angle, 2*math.pi)

    for i = 1, state.segments do
        local t = i / state.segments

        local a = start_angle + (sweep_angle * t)
        local ecx = rx * math.cos(a)
        local ecy = ry * math.sin(a)

        state.current_pos = rotate_around_axis(angle, ecx, ecy) + center
        insert_current_pos(state)
    end
    return state
end

local cmd_elliptic_arc_curve_abs = function(state, params)
    for i = 1, #params, 7 do
        local rx = math.abs(params[i])
        local ry = math.abs(params[i+1])
        local angle = signed_mod(params[i+2], 360) * math.pi / 180
        local large_arc_flag = params[i+3]
        local sweep_flag = params[i+4]
        local p0 = state.current_pos
        local p1 = state.pos + vector(params[i+5], params[i+6], 0) * state.size
        if p0 ~= p1 then
            state = elliptic_arc_to(state, rx, ry, angle, large_arc_flag, sweep_flag, p0, p1)
        end
    end
    return state
end

local cmd_elliptic_arc_curve = function(state, params)
    for i = 1, #params, 7 do
        local rx = math.abs(params[i])
        local ry = math.abs(params[i+1])
        local angle = signed_mod(params[i+2], 360) * math.pi / 180
        local large_arc_flag = params[i+3]
        local sweep_flag = params[i+4]
        local p0 = state.current_pos
        local p1 = state.current_pos + vector(params[i+5], params[i+6], 0) * state.size
        if p0 ~= p1 then
            state = elliptic_arc_to(state, rx, ry, angle, large_arc_flag, sweep_flag, p0, p1)
        end
    end
    return state
end

local all_commands = {
    Z = cmd_close_path,
    z = cmd_close_path,
    M = cmd_move_to_abs,
    m = cmd_move_to,
    L = cmd_line_to_abs,
    l = cmd_line_to,
    H = cmd_line_horizontal_abs,
    h = cmd_line_horizontal,
    V = cmd_line_vertical_abs,
    v = cmd_line_vertical,
    C = cmd_cubic_bezier_curve_abs,
    c = cmd_cubic_bezier_curve,
    S = cmd_smooth_cubic_bezier_curve_abs,
    s = cmd_smooth_cubic_bezier_curve,
    Q = cmd_quadratic_bezier_curve_abs,
    q = cmd_quadratic_bezier_curve,
    T = cmd_smooth_quadratic_bezier_curve_abs,
    t = cmd_smooth_quadratic_bezier_curve,
    A = cmd_elliptic_arc_curve_abs,
    a = cmd_elliptic_arc_curve,
}

-- A path_step represents a single path step.
--
-- There are 20 possible commands, broken up into 6 types,
-- with each command having an "absolute" (upper case) and
-- a "relative" (lower case) version.
--
-- ClosePath: Z, z
-- MoveTo: M, m
-- LineTo: L, l, H, h, V, v
-- Cubic Bézier Curve: C, c, S, s
-- Quadratic Bézier Curve: Q, q, T, t
-- Elliptical Arc Curve: A, a
--
-- The 'C' field is the command, and the 'P' field is the numeric parameters.
local process_path_step = function(state, path_step)
    local cmd = all_commands[path_step.C]
    if cmd == nil then
        return state  -- error - command not found
    end
    state = cmd(state, path_step.P)
    return state
end

NodeLibrary:addNodes(
    {
        SVGPath = {
            label = "SVG Path",
            op = function(inputs)
                local normal = V.normalize(inputs.normal)
                local right = V.normalize(inputs.right)
                local state = {
                    normal = normal,
                    right = right,
                    pos = inputs.pos,
                    current_pos = inputs.pos,
                    segments = inputs.segments,
                    size = inputs.size,
                    points = {},
                    mesh = nil,
                    last_cmd = nil,
                    last_p1 = nil,
                    last_p2 = nil,
                    last_p3 = nil,
                }

                if inputs.segments < 1 then
                    return {
                        out_mesh = state.mesh
                    }
                end

                local path_steps = parse_path(inputs.d)
                for _, path_step in pairs(path_steps) do
                    process_path_step(state, path_step)
                    state.last_cmd = path_step.C  -- used for smooth curves
                end

                terminate_path(state)

                return {
                    out_mesh = state.mesh
                }
            end,
            inputs = {
                P.lua_str("d"),
                P.v3("pos", vector(0, 0, 0)),
                P.v3("normal", vector(0, 1, 0)),
                P.v3("right", vector(1, 0, 0)),
                P.v3("size", vector(1, 1, 1)),
                P.scalar_int("segments", {default = 10, min = 0, soft_max = 360}),
            },
            outputs = {P.mesh("out_mesh")},
            returns = "out_mesh"
        }
    }
)
