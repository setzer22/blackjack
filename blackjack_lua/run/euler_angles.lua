local P = require("params")
local NodeLibrary = require("node_library")
local V = require("vector_math")

local euler_to_v3 = function(pitch, yaw)
    local phi = yaw * math.pi/180
    local cosPhi = math.cos(phi)
    local sinPhi = math.sin(phi)
    local theta = pitch * math.pi/180
    local cosTheta = math.cos(theta)
    local sinTheta = math.sin(theta)

    return vector(cosPhi*cosTheta, sinTheta, sinPhi*cosTheta)
end

NodeLibrary:addNodes(
    {
        EulerAngles = {
            label = "Euler Angles",
            op = function(inputs)
                local normal = euler_to_v3(inputs.pitch+90, inputs.yaw)
                local right = euler_to_v3(inputs.pitch, inputs.yaw)

                return {
                    normal = normal,
                    right = right,
                }
            end,
            inputs = {
                P.scalar("pitch", {default = 0, min = -360, soft_max = 360}),
                P.scalar("yaw", {default = 0, min = -360, soft_max = 360}),
            },
            outputs = {P.v3("normal"), P.v3("right")},
            returns = "normal"
        }
    }
)
