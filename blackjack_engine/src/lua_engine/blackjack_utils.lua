-- Copyright (C) 2022 setzer22 and contributors
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

return BlackjackUtils
