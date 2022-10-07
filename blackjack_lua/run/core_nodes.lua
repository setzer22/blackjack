-- Copyright (C) 2022 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local P = require("params")
local NodeLibrary = require("node_library")

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
            P.scalar("radius", { default = 1.0, min = 0.0 }),
            P.scalar_int("num_vertices", { default = 8, min = 3, soft_max = 32 }),
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
            P.scalar("radius", { default = 1.0, min = 0.0 }),
            P.scalar_int("segments", { default = 12, min = 3, soft_max = 64 }),
            P.scalar_int("rings", { default = 6, min = 3, soft_max = 64 }),
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
            P.scalar_int("segments", { default = 1, min = 1, soft_max = 32 }),
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
                out_heightmap = HeightMap.from_fn(inputs.width, inputs.height, f),
            }
        end,
        inputs = {
            P.scalar("width", { default = 100.0, min = 0.0, soft_max = 1000.0 }),
            P.scalar("height", { default = 100.0, min = 0.0, soft_max = 1000.0 }),
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
        return Types.VERTEX_ID
    elseif s == "Face" then
        return Types.FACE_ID
    elseif s == "Halfedge" then
        return Types.HALFEDGE_ID
    end
end
local function parse_ch_val(s)
    if s == "f32" then
        return Types.F32
    elseif s == "Vec3" then
        return Types.VEC3
    end
end

-- Edit ops: Nodes to edit existing meshes
local edit_ops = {
    BevelEdges = {
        label = "Bevel edges",
        inputs = {
            P.mesh("in_mesh"),
            P.selection("edges"),
            P.scalar("amount", { default = 0.0, min = 0.0, soft_max = 1.0 }),
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
            P.scalar("amount", { default = 0.0, min = 0.0, soft_max = 1.0 }),
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
            P.scalar("amount", { default = 0.0, min = 0.0, soft_max = 1.0 }),
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
            P.scalar_int("flip", { default = 0.0, min = 0.0, soft_max = 4.0 }),
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
            P.scalar_int("iterations", { default = 1, min = 0, soft_max = 7 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            if inputs.iterations < 1 then
                return { out_mesh = inputs.mesh:clone() }
            else
                return {
                    out_mesh = Ops.subdivide(inputs.mesh, inputs.iterations, inputs.technique == "catmull-clark"),
                }
            end
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
            Ops.vertex_attribute_transfer(inputs.src_mesh, out_mesh, Types.VEC3, inputs.channel)
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
            P.scalar_int("material_index", { default = 0, min = 0 }),
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
            P.scalar_int("flip", { default = 0.0, min = 0.0, soft_max = 4.0 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    ResampleCurve = {
        label = "Resample curve",
        op = function(inputs)
            return {
                out_mesh = Ops.resample_curve(
                    inputs.curve,
                    inputs.density_mode,
                    inputs.density,
                    inputs.tension,
                    inputs.alpha
                ),
            }
        end,
        inputs = {
            P.mesh("curve"),
            P.enum("density_mode", { "Uniform", "Curvature" }, 0),
            P.scalar("density", { default = 1.0, min = 0.05, soft_max = 10.0 }),
            P.scalar("tension", { default = 0.0, min = 0.0, max = 1.0 }),
            P.scalar("alpha", { default = 0.5, min = 0.0, max = 1.0 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    PointCloud = {
        label = "Point cloud",
        op = function(inputs)
            return { out_mesh = inputs.mesh:point_cloud(inputs.points) }
        end,
        inputs = {
            P.mesh("mesh"),
            P.selection("points"),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    -- TODO: This should be a more generic randomize channel
    RandomizeSize = {
        label = "Randomize size",
        op = function(inputs)
            local mesh = inputs.mesh:clone()
            local size_ch = mesh:ensure_channel(Types.VERTEX_ID, Types.F32, "size")
            math.randomseed(inputs.seed)
            for i = 0, #size_ch do
                size_ch[i] = math.random() * inputs.scale
            end
            mesh:set_channel(Types.VERTEX_ID, Types.F32, "size", size_ch)
            return { out_mesh = mesh }
        end,
        inputs = {
            P.mesh("mesh"),
            P.scalar("scale", { default = 1.0, soft_min = 0.0, soft_max = 2.0 }),
            P.scalar("seed", { default = 0.0 }),
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
            P.scalar("x", { default = 0.0 }),
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
            P.scalar("x", { default = 0.0 }),
            P.scalar("y", { default = 0.0 }),
            P.scalar("z", { default = 0.0 }),
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
            P.scalar("x", { default = 0 }),
            P.scalar("y", { default = 0 }),
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
        label = "Export OBJ",
        inputs = {
            P.mesh("mesh"),
            P.file("path"),
        },
        outputs = {},
        executable = true,
        op = function(inputs)
            HalfEdgeMesh.to_wavefront_obj(inputs.mesh, inputs.path)
        end,
    },
    ImportObj = {
        label = "Import OBJ",
        inputs = {
            P.file("path", "open")
        },
        outputs = {
            P.mesh("out_mesh")
        },
        op = function(inputs)
            local out_mesh = HalfEdgeMesh.from_wavefront_obj(inputs.path)
            return { out_mesh = out_mesh }
        end,
    }
}

NodeLibrary:addNodes(primitives)
NodeLibrary:addNodes(edit_ops)
NodeLibrary:addNodes(math_nodes)
NodeLibrary:addNodes(export)
