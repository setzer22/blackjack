-- Copyright (C) 2022 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local P = require("params")

local function load_function(code)
    local func, err = loadstring(code)
    if err ~= nil then
        error(err)
    end
    if typeof(func) ~= "function" then
        error("Code should be a single lua function")
    end
    return func
end

-- Primitives: Construct new meshes based on common patterns
local primitives = {
    MakeBox = {
        label = "Box",
        op = function(inputs)
            return { out_mesh = Primitives.cube(inputs.origin, inputs.size) }
        end,
        inputs = {
            P.v3("origin", vector(0, 0, 0)),
            P.v3("size", vector(1, 1, 1)),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    MakeQuad = {
        label = "Quad",
        op = function(inputs)
            return {
                out_mesh = Primitives.quad(inputs.center, inputs.normal, inputs.right, inputs.size),
            }
        end,
        inputs = {
            P.v3("center", vector(0, 0, 0)),
            P.v3("normal", vector(0, 1, 0)),
            P.v3("right", vector(1, 0, 0)),
            P.v3("size", vector(1, 1, 1)),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    MakeCircle = {
        label = "Circle",
        op = function(inputs)
            return { out_mesh = Primitives.circle(inputs.center, inputs.radius, inputs.num_vertices) }
        end,
        inputs = {
            P.v3("center", vector(0, 0, 0)),
            P.scalar("radius", 1.0, 0.0, 10.0),
            P.scalar("num_vertices", 8.0, 3.0, 32.0),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    MakeUVSphere = {
        label = "UV Sphere",
        op = function(inputs)
            return { out_mesh = Primitives.uv_sphere(inputs.center, inputs.radius, inputs.segments, inputs.rings) }
        end,
        inputs = {
            P.v3("center", vector(0, 0, 0)),
            P.scalar("radius", 1.0, 0.0, 10.0),
            P.scalar("segments", 12, 3, 64),
            P.scalar("rings", 6, 3, 64),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    MakeLine = {
        label = "Line",
        op = function(inputs)
            return { out_mesh = Primitives.line(inputs.start_point, inputs.end_point, inputs.segments) }
        end,
        inputs = {
            P.v3("start_point", vector(0, 0, 0)),
            P.v3("end_point", vector(0.0, 1.0, 0.0)),
            P.scalar("segments", 1, 1, 32),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    MakeTerrain = {
        label = "Terrain",
        op = function(inputs)
            local f = load_function(inputs.code)
            return {
                out_heightmap = Blackjack.heightmap_fn(inputs.width, inputs.height, f),
            }
        end,
        inputs = {
            P.scalar("width", 100.0, 0.0, 1000.0),
            P.scalar("height", 100.0, 0.0, 1000.0),
            P.lua_str("code"),
        },
        outputs = {
            P.heightmap("out_heightmap"),
        },
        returns = "out_heightmap",
    },
    MakeCode = {
        label = "Lua code",
        op = function(inputs)
            return {
                out_code = inputs.code,
            }
        end,
        inputs = {
            P.lua_str("code"),
        },
        outputs = {
            P.lua_str("out_code"),
        },
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
        return Types.f32
    elseif s == "Vec3" then
        return Types.Vec3
    end
end

-- Edit ops: Nodes to edit existing meshes
local edit_ops = {
    BevelEdges = {
        label = "Bevel edges",
        inputs = {
            P.mesh("in_mesh"),
            P.selection("edges"),
            P.scalar("amount", 0.0, 0.0, 1.0),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.bevel(inputs.edges, inputs.amount, out_mesh)
            return { out_mesh = out_mesh }
        end,
    },
    ChamferVertices = {
        label = "Chamfer vertices",
        inputs = {
            P.mesh("in_mesh"),
            P.selection("vertices"),
            P.scalar("amount", 0.0, 0.0, 1.0),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.chamfer(inputs.vertices, inputs.amount, out_mesh)
            return { out_mesh = out_mesh }
        end,
    },
    ExtrudeFaces = {
        label = "Extrude faces",
        inputs = {
            P.mesh("in_mesh"),
            P.selection("faces"),
            P.scalar("amount", 0.0, 0.0, 1.0),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.extrude(inputs.faces, inputs.amount, out_mesh)
            return { out_mesh = out_mesh }
        end,
    },
    BridgeLoops = {
        label = "Bridge Loops",
        inputs = {
            P.mesh("in_mesh"),
            P.selection("loop_1"),
            P.selection("loop_2"),
            P.scalar("flip", 0.0, 0.0, 10.0),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.bridge_chains(out_mesh, inputs.loop_1, inputs.loop_2, inputs.flip)
            return { out_mesh = out_mesh }
        end,
    },
    MakeQuadFace = {
        label = "Make face (quad)",
        inputs = {
            P.mesh("in_mesh"),
            P.selection("a"),
            P.selection("b"),
            P.selection("c"),
            P.selection("d"),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.make_quad(out_mesh, inputs.a, inputs.b, inputs.c, inputs.d)
            return { out_mesh = out_mesh }
        end,
    },
    MergeMeshes = {
        label = "Merge meshes",
        inputs = {
            P.mesh("mesh_a"),
            P.mesh("mesh_b"),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh_a:clone()
            Ops.merge(out_mesh, inputs.mesh_b)
            return { out_mesh = out_mesh }
        end,
    },
    Subdivide = {
        label = "Subdivide",
        inputs = {
            P.mesh("mesh"),
            P.enum("technique", { "linear", "catmull-clark" }, 0),
            P.scalar("iterations", 1, 1, 7),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            return {
                out_mesh = Ops.subdivide(inputs.mesh, inputs.iterations, inputs.technique == "catmull-clark"),
            }
        end,
    },
    SetNormals = {
        label = "Set Normals",
        inputs = {
            P.mesh("mesh"),
            P.enum("normals", { "smooth", "flat" }, 0),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh:clone()
            if inputs.normals == "smooth" then
                Ops.set_smooth_normals(out_mesh)
            else
                Ops.set_flat_normals(out_mesh)
            end
            return { out_mesh = out_mesh }
        end,
    },
    Transform = {
        label = "Transform",
        inputs = {
            P.mesh("mesh"),
            P.v3("translate", vector(0, 0, 0)),
            P.v3("rotate", vector(0, 0, 0)),
            P.v3("scale", vector(1, 1, 1)),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh:clone()
            Ops.transform(out_mesh, inputs.translate, inputs.rotate, inputs.scale)
            return { out_mesh = out_mesh }
        end,
    },
    VertexAttribTransfer = {
        label = "Vertex attribute transfer",
        inputs = {
            P.mesh("src_mesh"),
            P.mesh("dst_mesh"),
            P.strparam("channel", "", false),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.dst_mesh:clone()
            Ops.vertex_attribute_transfer(inputs.src_mesh, out_mesh, Types.Vec3, inputs.channel)
            return { out_mesh = out_mesh }
        end,
    },
    SetFullRangeUVs = {
        label = "Set full range UVs",
        inputs = {
            P.mesh("mesh"),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh:clone()
            Ops.set_full_range_uvs(out_mesh)
            return { out_mesh = out_mesh }
        end,
    },
    SetMaterial = {
        label = "Set material",
        inputs = {
            P.mesh("mesh"),
            P.selection("faces"),
            P.scalar("material_index", 0.0, 0.0, 5.0),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh:clone()
            Ops.set_material(out_mesh, inputs.faces, inputs.material_index)
            return { out_mesh = out_mesh }
        end,
    },
    MakeGroup = {
        label = "Make group",
        inputs = {
            P.mesh("mesh"),
            P.enum("type", { "Vertex", "Face", "Halfedge" }, 0),
            P.strparam("name", ""),
            P.selection("selection"),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh:clone()
            local typ = parse_ch_key(inputs.type)
            Ops.make_group(out_mesh, typ, inputs.selection, inputs.name)
            return { out_mesh = out_mesh }
        end,
    },
    EditChannels = {
        label = "Edit channels",
        inputs = {
            P.mesh("mesh"),
            P.enum("channel_key", { "Vertex", "Face", "Halfedge" }, 0),
            P.strparam("channels", ""),
            P.lua_str("code"),
        },

        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.mesh:clone()
            local k_typ = parse_ch_key(inputs.channel_key)

            local func, err = loadstring(inputs.code)
            if err ~= nil then
                error(err)
                return
            end
            if typeof(func) ~= "function" then
                error("Code should be a single lua function")
            end

            local ch_size = 0
            local ch_by_name = {}
            for ch_descr in inputs.channels:gmatch("[^,]+") do
                local _, _, ch_name, ch_val_str = ch_descr:find("(%w+)%s*:%s*(%w+)")
                local val_typ = parse_ch_val(ch_val_str)
                local ch_data = out_mesh:ensure_channel(k_typ, val_typ, ch_name)
                ch_size = #ch_data
                ch_by_name[ch_name] = { data = ch_data, value_type = val_typ }
            end

            for i = 1, ch_size do
                local ch_i_map = { index = i }
                for ch_name, ch in ch_by_name do
                    ch_i_map[ch_name] = ch.data[i]
                end
                local ch_i_out = func(ch_i_map)
                for ch_name, val in ch_i_out do
                    local ch = ch_by_name[ch_name]
                    if ch ~= nil then
                        ch.data[i] = val
                    end
                end
            end

            for ch_name, ch in ch_by_name do
                out_mesh:set_channel(k_typ, ch.value_type, ch_name, ch.data)
            end

            return { out_mesh = out_mesh }
        end,
    },
    CopyToPoints = {
        label = "Copy to points",
        op = function(inputs)
            return { out_mesh = Ops.copy_to_points(inputs.points, inputs.mesh) }
        end,
        inputs = {
            P.mesh("points"),
            P.mesh("mesh"),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    ExtrudeAlongCurve = {
        label = "Extrude along curve",
        op = function(inputs)
            return { out_mesh = Ops.extrude_along_curve(inputs.backbone, inputs.cross_section, inputs.flip) }
        end,
        inputs = {
            P.mesh("backbone"),
            P.mesh("cross_section"),
            P.scalar("flip", 0.0, 0.0, 10.0),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
}

-- Math: Nodes to perform vector or scalar math operations
local math_nodes = {
    MakeScalar = {
        label = "Scalar",
        inputs = {
            P.scalar("x", 0.0, 0.0, 2.0),
        },
        outputs = {
            P.scalar("x"),
        },
        op = function(inputs)
            return { x = inputs.x }
        end,
    },
    MakeVector = {
        label = "MakeVector",
        inputs = {
            P.scalar("x", 0.0, -100.0, 100.0),
            P.scalar("y", 0.0, -100.0, 100.0),
            P.scalar("z", 0.0, -100.0, 100.0),
        },
        outputs = {
            P.v3("v"),
        },
        op = function(inputs)
            return { v = vector(inputs.x, inputs.y, inputs.z) }
        end,
    },
    VectorMath = {
        label = "Vector math",
        inputs = {
            P.enum("op", { "Add", "Sub", "Mul" }, 0),
            P.v3("vec_a", vector(0, 0, 0)),
            P.v3("vec_b", vector(0, 0, 0)),
        },
        outputs = {
            P.v3("out"),
        },
        op = function(inputs)
            local out
            if inputs.op == "Add" then
                out = inputs.vec_a + inputs.vec_b
            elseif inputs.op == "Sub" then
                out = inputs.vec_a - inputs.vec_b
            elseif inputs.op == "Mul" then
                out = inputs.vec_a * inputs.vec_b
            end
            return { out = out }
        end,
    },
    ScalarMath = {
        label = "Scalar math",
        inputs = {
            P.enum("op", { "Add", "Sub", "Mul" }, 0),
            P.scalar("x", 0, -100.0, 100.0),
            P.scalar("y", 0, -100.0, 100.0),
        },
        outputs = {
            P.scalar("out"),
        },
        op = function(inputs)
            local out
            if inputs.op == "Add" then
                out = inputs.x + inputs.y
            elseif inputs.op == "Sub" then
                out = inputs.x - inputs.y
            elseif inputs.op == "Mul" then
                out = inputs.x * inputs.y
            end
            return { out = out }
        end,
    },
}

-- Export: Nodes to export the generated meshes outside of blacjack
local export = {
    ExportObj = {
        label = "Export obj",
        inputs = {
            P.mesh("mesh"),
            P.file("path"),
        },
        outputs = {},
        executable = true,
        op = function(inputs)
            Export.wavefront_obj(inputs.mesh, inputs.path)
        end,
    },
}

NodeLibrary:addNodes(primitives)
NodeLibrary:addNodes(edit_ops)
NodeLibrary:addNodes(math_nodes)
NodeLibrary:addNodes(export)