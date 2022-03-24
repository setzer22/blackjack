local vec4 = {}
vec4.__index = vec4

local function new(x, y, z, w)
    return setmetatable({x = x, y = y, z = z, w = w}, vec4)
end

-- check if an object is a vec4
local function isvec4(t) return getmetatable(t) == vec4 end

-- meta function to add vec4s
function vec4.__add(a, b)
    assert(isvec4(a) and isvec4(b),
           "add: wrong argument types: (expected <vec4> and <vec4>)")
    return new(a.x + b.x, a.y + b.y, a.z + b.z, a.w + b.w)
end

-- meta function to subtract vec4s
function vec4.__sub(a, b)
    assert(isvec4(a) and isvec4(b),
           "sub: wrong argument types: (expected <vec4> and <vec4>)")
    return new(a.x - b.x, a.y - b.y, a.z - b.z, a.w - b.w)
end

-- meta function to multiply vec4s
function vec4.__mul(a, b)
    if type(a) == 'number' then
        return new(a * b.x, a * b.y, a * b.z, a * b.w)
    elseif type(b) == 'number' then
        return new(a.x * b, a.y * b, a.z * b, a.w * b)
    else
        assert(isvec4(a) and isvec4(b),
               "mul: wrong argument types: (expected <vec4> or <number>)")
        return new(a.x * b.x, a.y * b.y, a.z * b.z, a.w + b.w)
    end
end

function vec4:__tostring()
    return "vec4(" .. self.x .. ", " .. self.y .. ", " .. self.z .. ", " ..  self.w ")"
end

function vec4:clone() return new(self.x, self.y, self.z, self.w) end

local module = {new = new, isvec4 = isvec4}
return setmetatable(module, {__call = function(_, ...) return new(...) end})
