-- Copyright (C) 2022 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local TableHelpers = {}

--- Reverses the sequential part of this table.
TableHelpers.reverse = function(t)
    local n = #t
    for i = 1, n do
        t[i], t[n] = t[n], t[i]
        n = n - 1
    end
    return t
end

return TableHelpers
