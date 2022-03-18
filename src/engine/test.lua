local function vec3(x, y, z) return {x = x, y = y, z = z} end

function Plugin_main()
    local mesh = Primitives.cube(vec3(0, 0, 0), vec3(2, 2, 2))
    local beveled = Ops.bevel(Blackjack.selection("*"), 0.1, mesh)
    return beveled
end
