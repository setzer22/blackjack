local vec2 = {}
vec2.__index = vec2

local function new(x, y) return setmetatable({x = x, y = y}, vec2) end

-- check if an object is a vec2
local function isvec2(t) return getmetatable(t) == vec2 end

-- meta function to add vec2s
function vec2.__add(a, b)
    assert(isvec2(a) and isvec2(b),
           "add: wrong argument types: (expected <vec2> and <vec2>)")
    return new(a.x + b.x, a.y + b.y)
end

-- meta function to subtract vec2s
function vec2.__sub(a, b)
    assert(isvec2(a) and isvec2(b),
           "sub: wrong argument types: (expected <vec2> and <vec2>)")
    return new(a.x - b.x, a.y - b.y)
end

-- meta function to multiply vec2s
function vec2.__mul(a, b)
    if type(a) == 'number' then
        return new(a * b.x, a * b.y)
    elseif type(b) == 'number' then
        return new(a.x * b, a.y * b)
    else
        assert(isvec2(a) and isvec2(b),
               "mul: wrong argument types: (expected <vec2> or <number>)")
        return new(a.x * b.x, a.y * b.y)
    end
end

function vec2:__tostring() return "vec2(" .. self.x .. ", " .. self.y .. ")" end

function vec2:clone() return new(self.x, self.y) end

local module = {new = new, isvec2 = isvec2}
return setmetatable(module, {__call = function(_, ...) return new(...) end})
