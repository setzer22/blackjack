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
local function strparam(name, default, multiline)
    return {name = name, default = default, type = "string", multiline = multiline}
end
local function lua_str(name)
    return {name = name, type = "lua_string"}
end
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
    MakeUVSphere = {
         label = "UV Sphere",
        op = function(inputs)
            return {out_mesh = Primitives.uv_sphere(inputs.center, inputs.radius, inputs.segments, inputs.rings)}
        end,
        inputs = {v3("center", vector(0,0,0)), scalar("radius", 1.0, 0.0, 10.0), scalar("segments", 12, 3, 64), scalar("rings", 6, 3, 64)},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    },
    MakeLine = {
         label = "Line",
        op = function(inputs)
            return {out_mesh = Primitives.line(inputs.start_point, inputs.end_point, inputs.segments)}
        end,
        inputs = {v3("start_point", vector(0,0,0)), v3("end_point", vector(0.0, 1.0, 0.0)), scalar("segments", 1, 1, 32)},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh"
    },
}

local function parse_ch_key(s)
    if s == "Vertex" then
        return Types.VertexId
    elseif s == "Face" then
        return Types.FaceId
    elseif s == "Halfedge" then
        return Types.HalfEdgeId
    end
end
local function parse_ch_val(s)
    if s == "f32" then
        return Types.F32
    elseif s == "Vec3" then
        return Types.Vec3
    end
end

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
            mesh("in_mesh"), selection("loop_1"), selection("loop_2"), scalar("flip", 0.0, 0.0, 10.0)
        },
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.bridge_chains(out_mesh, inputs.loop_1, inputs.loop_2, inputs.flip)
            return {out_mesh = out_mesh}
        end
    },
    MakeQuadFace = {
        label = "Make face (quad)",
        inputs = {
            mesh("in_mesh"), selection("a"), selection("b"), selection("c"), selection("d"),
        },
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.make_quad(out_mesh, inputs.a, inputs.b, inputs.c, inputs.d)
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
            if inputs.normals == "smooth" then
                Ops.set_smooth_normals(out_mesh)
            else
                Ops.set_flat_normals(out_mesh)
            end
            return {out_mesh = out_mesh}
        end
    },
    Transform = {
        label = "Transform",
        inputs = {
                    mesh("mesh"), 
                    v3("translate", vector(0, 0, 0)),
                    v3("rotate", vector(0, 0, 0)),
                    v3("scale", vector(1, 1, 1))
                },
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh:clone()
            Ops.transform(out_mesh, inputs.translate, inputs.rotate, inputs.scale)
            return {out_mesh = out_mesh}
        end
    },
    VertexAttribTransfer = {
        label = "Vertex attribute transfer",
        inputs = {
                    mesh("src_mesh"), 
                    mesh("dst_mesh"),
                    strparam("channel", "", false)
                },
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.dst_mesh:clone()
            Ops.vertex_attribute_transfer(inputs.src_mesh, out_mesh, Types.Vec3, inputs.channel)
            return {out_mesh = out_mesh}
        end
    },
    SetFullRangeUVs = {
        label = "Set full range UVs",
        inputs = {mesh("mesh")},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh:clone()
            Ops.set_full_range_uvs(out_mesh);
            return {out_mesh = out_mesh}
        end
    },
    SetMaterial = {
        label = "Set material",
        inputs = {mesh("mesh"), selection("faces"), scalar("material_index", 0.0, 0.0, 5.0)},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh:clone()
            Ops.set_material(out_mesh, inputs.faces, inputs.material_index);
            return {out_mesh = out_mesh}
        end
    },
    MakeGroup = {
        label = "Make group",
        inputs = {mesh("mesh"), enum("type", {"Vertex", "Face", "Halfedge"}, 0), strparam("name", ""), selection("selection")},
        outputs = {mesh("out_mesh")},
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh:clone()
            local typ = parse_ch_key(inputs.type)
            Ops.make_group(out_mesh, typ, inputs.selection, inputs.name);
            return {out_mesh = out_mesh}
        end
    },
    EditChannels = {
        label = "Edit channels",
        inputs = { 
            mesh("mesh"),
            enum("channel_key", {"Vertex", "Face", "Halfedge"}, 0),
            enum("channel_val", {"f32", "Vec3"}, 0),
            strparam("channels", ""),
            lua_str("code"),
        },

        outputs = { mesh("out_mesh") },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh:clone()
            local typ = parse_ch_key(inputs.channel_key) 
            local val = parse_ch_val(inputs.channel_val) 

            local func, err = loadstring(inputs.code)
            if err ~= nil then
                error(err)
                return
            end
            if typeof(func) ~= 'function' then
                error('Code should be a single lua function')
            end

            local ch_size = 0
            local ch_by_name = {}
            for ch_name in inputs.channels:gmatch("[^, ]+") do
                -- TODO: The `val` in get_channel could be inferred.
                local ch = out_mesh:ensure_channel(typ, val, ch_name)
                ch_size = #ch
                ch_by_name[ch_name] = ch
            end

            for i = 1,ch_size do
                local ch_i_map = { index = i }
                for ch_name, ch in ch_by_name do
                    ch_i_map[ch_name] = ch[i]
                end
                local ch_i_out = func(ch_i_map)
                for ch_name, val in ch_i_out do
                    local ch = ch_by_name[ch_name]
                    if ch ~= nil then
                        ch_by_name[ch_name][i] = val
                    end
                end
            end

            for ch_name, ch in ch_by_name do
                out_mesh:set_channel(typ, val, ch_name, ch)
            end
            
            return { out_mesh = out_mesh }
        end
    }
}

-- Math: Nodes to perform vector or scalar math operations
local math = {
    MakeScalar = {
        label = "Scalar",
        inputs = {
            scalar("x", 0.0, 0.0, 2.0),
        },
        outputs = {scalar("x")},
        op = function(inputs)
            return {x = inputs.x}
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
