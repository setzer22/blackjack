-- Copyright (C) 2023 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local BlackjackUtils = {}

BlackjackUtils.load_function = function(code)
    local func, err = loadstring(code)
    if err ~= nil then
        error(err)
    end
    if typeof(func) ~= "function" then
        error("Code should be a single lua function")
    end
    return func
end

BlackjackUtils.parse_ch_key = function(s)
    if s == "Vertex" then
        return Types.VERTEX_ID
    elseif s == "Face" then
        return Types.FACE_ID
    elseif s == "Halfedge" then
        return Types.HALFEDGE_ID
    end
end
BlackjackUtils.parse_ch_val = function(s)
    if s == "f32" then
        return Types.F32
    elseif s == "Vec3" then
        return Types.VEC3
    end
end

return BlackjackUtils
