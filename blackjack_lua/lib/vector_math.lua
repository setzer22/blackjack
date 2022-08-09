local VectorMath = {}

VectorMath.distance = function(v1, v2)
  local d = v2 - v1
  return math.sqrt(d.x * d.x + d.y * d.y + d.z * d.z)
end

VectorMath.distance_squared = function(v1, v2)
  local d = v2 - v1
  return d.x * d.x + d.y * d.y + d.z * d.z
end

VectorMath.floor = function(v)
  return vector(math.floor(v.x), math.floor(v.y), math.floor(v.z))
end

return VectorMath
