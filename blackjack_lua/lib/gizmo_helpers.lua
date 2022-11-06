-- Copyright (C) 2022 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local GizmoHelpers = {}

--- Takes a list of input parameter names representing points in space, and
--- returns a gizmo description that will allow tweaking each of these points.
GizmoHelpers.tweak_points = function(point_params)
    return {
        update_params = function(inputs, gizmos)
            for i, point_param in point_params do
                local gizmo = gizmos[i]
                if gizmo ~= nil then
                    inputs[point_param] = gizmo:translation()
                end
            end
            return inputs
        end,
        pre_op = function(_inputs)
            return {}
        end,
        update_gizmos = function(inputs, gizmos, _outputs)
            if gizmos ~= nil then
                for i, point_param in point_params do
                    gizmos[i]:set_translation(inputs[point_param])
                end
                return gizmos
            else
                local new_gizmos = {}
                for i, point_param in point_params do
                    table.insert(new_gizmos, TransformGizmo.new(inputs[point_param], vector(0, 0, 0), vector(1, 1, 1)))
                end
                return new_gizmos
            end
        end,
    }
end

return GizmoHelpers
