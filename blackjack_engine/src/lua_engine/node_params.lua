-- Copyright (C) 2023 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local Params = {}

--- A scalar parameter, with given `default`, `min` and `max` value
Params.scalar = function(name, config)
    if config ~= nil then
        assert(type(config) == 'table', "config should be table")
        return {
            name = name,
            default = config.default or 0.0,
            min = config.min,
            max = config.max,
            soft_min = config.soft_min,
            soft_max = config.soft_max,
            type = "scalar",
        }
    else
        return { name = name, type = "scalar" }
    end
end

Params.scalar_int = function(name, config)
    local s = Params.scalar(name, config)
    s.num_decimals = 0
    return s
end

--- A vector parameter, with given `default` value
Params.v3 = function(name, default)
    return { name = name, default = default, type = "vec3" }
end

--- A mesh parameter. Meshes can't be set by the user directly via widget, so
--- this has no additional settings.
Params.mesh = function(name)
    return { name = name, type = "mesh" }
end

--- A selection parameter. Lets user specify a group of vertices, halfedges or
--- faces. The selected element is context-dependent.
Params.selection = function(name)
    return { name = name, type = "selection" }
end

--- A string parameter, with a given `default` value. If `multiline` is set, the
--- widget for this parameter will allow inserting newlines.
Params.strparam = function(name, default, multiline)
    return { name = name, default = default, type = "string", multiline = multiline }
end

--- A special string parameter made to contain lua source code. The widget for
--- this parameter supports syntax highlighting.
Params.lua_str = function(name)
    return { name = name, type = "lua_string" }
end

--- Another special string parameter, which lets the user select among a given
--- set of pre-defined `values`. The `selected` parameter may be used to
--- optionally provide the index of the default selection.
Params.enum = function(name, values, selected)
    return {
        name = name,
        type = "enum",
        values = values or {},
        selected = selected,
    }
end

--- A file parameter. Internally handled as a string, but shows a file picker
--- widget on the UI.
---
--- The `mode` specifies whether the file picker is used to create a new file
--- with `"save"` or open an existing one with `"open"`
Params.file = function(name, mode)
    mode = mode or "save" -- keep backwards compatibility
    return { name = name, type = "file", mode = mode }
end

--- A heightmap mesh parameter. Like a regular mesh, it can't be set by the user
--- so it has no widget.
Params.heightmap = function(name)
    return { name = name, type = "heightmap" }
end

return Params
