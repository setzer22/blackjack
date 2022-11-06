-- Copyright (C) 2022 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local GizmoHelpers = {}

--- Takes a list of input parameter names representing points in space, and
--- returns a gizmo description that will allow tweaking each of these points.
GizmoHelpers.tweak_points = function(point_params)
    local gizmo_descriptors = {}
    for i, point_param in point_params do
        local gizmo_descr = {
            update_params = function(inputs, gizmo)
                inputs[point_param] = gizmo:translation()
                return inputs
            end,
            pre_op = function(_inputs)
                return {}
            end,
            update_gizmos = function(inputs, gizmo, _outputs)
                if gizmo ~= nil then
                    gizmo:set_translation(inputs[point_param])
                    return gizmo
                else
                    return TransformGizmo.new(inputs[point_param], vector(0, 0, 0), vector(1, 1, 1))
                end
            end,
            affected_params = function()
                return { point_param }
            end
        }
        table.insert(gizmo_descriptors, gizmo_descr)
    end
    return gizmo_descriptors
end

return GizmoHelpers
