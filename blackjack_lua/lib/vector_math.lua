-- Copyright (C) 2023 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local VectorMath = {}

VectorMath.length = function(v)
    return math.sqrt(v.x * v.x + v.y * v.y + v.z * v.z)
end

VectorMath.normalize = function(v)
    return v * (1.0 / VectorMath.length(v))
end

VectorMath.distance = function(v1, v2)
    local d = v2 - v1
    return math.sqrt(d.x * d.x + d.y * d.y + d.z * d.z)
end

VectorMath.distance_squared = function(v1, v2)
    local d = v2 - v1
    return d.x * d.x + d.y * d.y + d.z * d.z
end

VectorMath.floor = function(v)
    return vector(math.floor(v.x), math.floor(v.y), math.floor(v.z))
end

VectorMath.display = function(v)
    return tostring(v.x) .. "," .. tostring(v.y) .. "," .. tostring(v.z)
end

VectorMath.from_string = function(s)
    local _, _, x, y, z = s:find("([^,]+),([^,]+),([^,]+)")
    if x and y and z then
        return vector(x, y, z)
    else
        error("Invalid vector format " .. s)
    end
end

VectorMath.dot = function(v1, v2)
    return v1.x * v2.x + v1.y * v2.y + v1.z * v2.z
end

VectorMath.cross = function(v, v2)
    return NativeMath.cross(v, v2)
end

VectorMath.rotate_around_axis = function(v, axis, angle)
    return NativeMath.rotate_around_axis(v, axis, angle)
end

return VectorMath
