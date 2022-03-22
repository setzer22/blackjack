local function scalar(name, default, min, max)
    return {
        name = name,
        default = default,
        min = min,
        max = max,
        type = "scalar"
    }
end
local function v3(name, default)
    return {name = name, default = default, type = "vec3"}
end
local function mesh(name) return {name = name, type = "mesh"} end
local function selection(name) return {name = name, type = "selection"} end

local core_nodes = {
    MakeBox = {
        label = "Box",
        op = function(inputs)
            return {out_mesh = Primitives.cube(inputs.origin, inputs.size)}
        end,
        inputs = {v3("origin", Vec3(0, 0, 0)), v3("size", Vec3(1, 1, 1))},
        outputs = {mesh("out_mesh")}
    },
    MakeQuad = {
        label = "Quad",
        op = function(inputs)
            return {
                out_mesh = Primitives.quad(inputs.center, inputs.normal,
                                           inputs.right, inputs.size)
            }
        end,
        inputs = {
            v3("center", Vec3(0, 0, 0)), v3("normal", Vec3(0, 1, 0)),
            v3("right", Vec3(1, 0, 0)), v3("size", Vec3(1, 1, 1))
        },
        outputs = {mesh("out_mesh")}
    },
    BevelEdges = {
        label = "Bevel edges",
        inputs = {
            mesh("in_mesh"), selection("edges"), scalar("amount", 0.0, 0.0, 1.0)
        },
        outputs = {mesh("out_mesh")},
        op = function(inputs)
            return {
                out_mesh = Ops.bevel(inputs.edges, inputs.amount, inputs.mesh)
            }
        end
    },
    MakeVector = {
        label = "MakeVector",
        inputs = {
            scalar("x", 0.0, -100.0, 100.0), scalar("y", 0.0, -100.0, 100.0),
            scalar("z", 0.0, -100.0, 100.0)
        },
        outputs = {v3("v")},
        op = function(inputs)
            return {v = Vec3(inputs.x, inputs.y, inputs.z)}
        end
    },
    Foo = {
        label = "Foo",
        inputs = {},
        outputs = {v3("v")},
        op = function(inputs) return {v = Vec3(2, 2, 2)} end
    },
    Bar = {
        label = "Bar",
        inputs = {v3("v", Vec3(1, 1, 1))},
        outputs = {mesh("out_mesh")},
        op = function(inputs)
            return {out_mesh = Primitives.cube(inputs.v, Vec3(1, 1, 1))}
        end
    }
}

NodeLibrary:addNodes(core_nodes)
