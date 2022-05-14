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
local function enum(name, values, selected)
    return {
        name = name,
        type = "enum",
        values = values or {},
        selected = selected
    }
end
local function file(name) return {name = name, type = "file"} end

-- Primitives: Construct new meshes based on common patterns
local primitives = {
    MakeBox = {
        label = "Box",
        op = function(inputs)
            return {out_mesh = Primitives.cube(inputs.origin, inputs.size)}
        end,
        inputs = {v3("origin", vector(0, 0, 0)), v3("size", vector(1, 1, 1))},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
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
            v3("center", vector(0, 0, 0)), v3("normal", vector(0, 1, 0)),
            v3("right", vector(1, 0, 0)), v3("size", vector(1, 1, 1))
        },
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    },
    MakeCircle = {
         label = "Circle",
        op = function(inputs)
            return {out_mesh = Primitives.circle(inputs.center, inputs.radius, inputs.num_vertices)}
        end,
        inputs = {v3("center", vector(0,0,0)), scalar("radius", 1.0, 0.0, 10.0), scalar("num_vertices", 8.0, 3.0, 32.0)},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    },
}

-- Edit ops: Nodes to edit existing meshes
local edit_ops = {
    BevelEdges = {
        label = "Bevel edges",
        inputs = {
            mesh("in_mesh"), selection("edges"), scalar("amount", 0.0, 0.0, 1.0)
        },
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.bevel(inputs.edges, inputs.amount, out_mesh)
            return {out_mesh = out_mesh}
        end
    },
    ChamferVertices = {
        label = "Chamfer vertices",
        inputs = {
            mesh("in_mesh"), selection("vertices"),
            scalar("amount", 0.0, 0.0, 1.0)
        },
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.chamfer(inputs.vertices, inputs.amount, out_mesh)
            return {out_mesh = out_mesh}
        end
    },
    ExtrudeFaces = {
        label = "Extrude faces",
        inputs = {
            mesh("in_mesh"), selection("faces"), scalar("amount", 0.0, 0.0, 1.0)
        },
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.extrude(inputs.faces, inputs.amount, out_mesh)
            return {out_mesh = out_mesh}
        end
    },
    BridgeLoops = {
        label = "Bridge Loops",
        inputs = {
            mesh("in_mesh"), selection("loop_1"), selection("loop_2") 
        },
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.bridge_loops(out_mesh, inputs.loop_1, inputs.loop_2)
            return {out_mesh = out_mesh}
        end
    },
    MergeMeshes = {
        label = "Merge meshes",
        inputs = {mesh("mesh_a"), mesh("mesh_b")},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh_a:clone()
            Ops.merge(out_mesh, inputs.mesh_b)
            return {out_mesh = out_mesh}
        end
    },
    Subdivide = {
        label = "Subdivide",
        inputs = {
            mesh("mesh"), enum("technique", {"linear", "catmull-clark"}, 0),
            scalar("iterations", 1, 1, 7)
        },
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            return {
                out_mesh = Ops.subdivide(inputs.mesh, inputs.iterations,
                                         inputs.technique == "catmull-clark")
            }
        end
    },
    SetNormals = {
        label = "Set Normals",
        inputs = {mesh("mesh"), enum("normals", {"smooth", "flat"}, 0)},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh:clone()
            print(inputs.normals)
            if inputs.normals == "smooth" then
                Ops.set_smooth_normals(out_mesh)
            else
                Ops.set_flat_normals(out_mesh)
            end
            return {out_mesh = out_mesh}
        end
    }
}

-- Math: Nodes to perform vector or scalar math operations
local math = {
    MakeVector = {
        label = "MakeVector",
        inputs = {
            scalar("x", 0.0, -100.0, 100.0), scalar("y", 0.0, -100.0, 100.0),
            scalar("z", 0.0, -100.0, 100.0)
        },
        outputs = {v3("v")},
        op = function(inputs)
            return {v = vector(inputs.x, inputs.y, inputs.z)}
        end
    },
    VectorMath = {
        label = "Vector math",
        inputs = {
            enum("op", {"Add", "Sub", "Mul"}, 0), v3("vec_a", vector(0, 0, 0)),
            v3("vec_b", vector(0, 0, 0))
        },
        outputs = {v3("out")},
        op = function(inputs)
            local out
            if inputs.op == "Add" then
                out = inputs.vec_a + inputs.vec_b
            elseif inputs.op == "Sub" then
                out = inputs.vec_a - inputs.vec_b
            elseif inputs.op == "Mul" then
                out = inputs.vec_a * inputs.vec_b
            end
            return {out = out}
        end
    }
}

-- Export: Nodes to export the generated meshes outside of blacjack
local export = {
    ExportObj = {
        label = "Export obj",
        inputs = {mesh("mesh"), file("path")},
        outputs = {},
        executable = true,
        op = function(inputs)
            Export.wavefront_obj(inputs.mesh, inputs.path)
        end
    }
}

NodeLibrary:addNodes(primitives)
NodeLibrary:addNodes(edit_ops)
NodeLibrary:addNodes(math)
NodeLibrary:addNodes(export)
