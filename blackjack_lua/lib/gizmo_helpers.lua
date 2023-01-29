-- Copyright (C) 2023 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local GizmoHelpers = {}

--- A gizmo that allows tweaking a single input parameter `point_param` as a
--- point in 3d space.
GizmoHelpers.tweak_point = function(point_param)
    return {
        -- Called when a gizmo has changed in the UI. This function needs to
        -- set the parameters of the node according to the gizmo.
        update_params = function(inputs, gizmo)
            inputs[point_param] = gizmo:translation()
            return inputs
        end,
        -- Called after op. This function must return a gizmo for this node, to
        -- be applied for the next frame. Ghe `gizmo` variable will contain the
        -- existing gizmo for the current frame, if any.
        update_gizmos = function(inputs, gizmo, _outputs)
            if gizmo ~= nil then
                gizmo:set_translation(inputs[point_param])
                return gizmo
            else
                local new_gizmo =
                    TransformGizmo.new(inputs[point_param], vector(0, 0, 0), vector(1, 1, 1))
                new_gizmo:set_enable_rotation(false)
                new_gizmo:set_enable_scale(false)
                return new_gizmo
            end
        end,
        --- Must return a list of parameter name lists, informing the engine of
        --- which params are affected by each gizmo. If all the parameters in
        --- this list have incoming connections, this gizmo will be skipped.
        affected_params = function()
            return { point_param }
        end,
    }
end


--- A gizmo that allows translating, rotating and scaling something in 3d space.
--- The optional `opts` argument can pass extra keys
--- `pre_{translation,rotation,scale}_param` to set a pre-transform. The
--- pre-transform affects the position of the gizmo, but not the parameter.
GizmoHelpers.tweak_transform = function(translation_param, rotation_param, scale_param, opts)
    local opts = opts or {}
    return {
        update_params = function(inputs, gizmo)
            inputs[translation_param] = gizmo:translation()
            inputs[rotation_param] = gizmo:rotation()
            inputs[scale_param] = gizmo:scale()
            return inputs
        end,
        update_gizmos = function(inputs, gizmo, _outputs)
            if gizmo == nil then
                gizmo = TransformGizmo.default()
            end
            gizmo:set_translation(inputs[translation_param])
            gizmo:set_rotation(inputs[rotation_param])
            gizmo:set_scale(inputs[scale_param])

            if opts.pre_translation_param ~= nil then
                gizmo:set_pre_translation(inputs[opts.pre_translation_param])
            end
            if opts.pre_rotation_param ~= nil then
                gizmo:set_pre_rotation(inputs[opts.pre_rotation_param])
            end
            if opts.pre_scale_param ~= nil then
                gizmo:set_pre_rotation(inputs[opts.pre_scale_param])
            end

            return gizmo
        end,
        affected_params = function()
            return { translation_param, rotation_param, scale_param }
        end,
    }
end

return GizmoHelpers
