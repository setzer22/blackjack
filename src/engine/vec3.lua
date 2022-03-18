local vec3 = {}
vec3.__index = vec3

local function new(x, y, z)
    return setmetatable({x = x, y = y, z = z}, vec3)
end

-- check if an object is a vec3
local function isvec3(t)
  return getmetatable(t) == vec3
end

-- meta function to add vec3s
function vec3.__add(a,b)
  assert(isvec3(a) and isvec3(b), "add: wrong argument types: (expected <vec3> and <vec3>)")
  return new(a.x+b.x, a.y+b.y, a.z+b.z)
end

-- meta function to subtract vec3s
function vec3.__sub(a,b)
  assert(isvec3(a) and isvec3(b), "sub: wrong argument types: (expected <vec3> and <vec3>)")
  return new(a.x-b.x, a.y-b.y, a.z-b.z)
end

-- meta function to multiply vec3s
function vec3.__mul(a,b)
  if type(a) == 'number' then 
    return new(a * b.x, a * b.y, a * b.z)
  elseif type(b) == 'number' then
    return new(a.x * b, a.y * b, a.z * b)
  else
    assert(isvec3(a) and isvec3(b),  "mul: wrong argument types: (expected <vec3> or <number>)")
    return new(a.x*b.x, a.y*b.y)
  end
end

function vec3:__tostring()
    return "Vec3("..self.x..", "..self.y..", "..self.z..")"
end

function vec3:clone()
    return new(self.x, self.y, self.z)
end

local module = {
    new = new,
    isvec3 = isvec3
}
return setmetatable(module, { __call = function(_,...) return new(...) end })