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
