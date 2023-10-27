-- Copyright (C) 2023 setzer22 and contributors
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local P = require("params")
local V = require("vector_math")
local T = require("table_helpers")
local Gz = require("gizmo_helpers")
local NodeLibrary = require("node_library")
local Utils = require("utils")

-- Primitives: Construct new meshes based on common patterns
local primitives = {
    MakeBox = {
        label = "Box",
        op = function(inputs)
            return {
                out_mesh = Primitives.cube(inputs.origin, inputs.size),
            }
        end,
        inputs = {
            P.v3("origin", vector(0, 0, 0)),
            P.v3("size", vector(1, 1, 1)),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        gizmos = { Gz.tweak_point("origin") },
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
        gizmos = { Gz.tweak_point("center") },
        returns = "out_mesh",
    },
    MakeCircle = {
        label = "Circle",
        op = function(inputs)
            return {
                out_mesh = Primitives.circle(
                    inputs.center,
                    inputs.radius,
                    inputs.num_vertices,
                    inputs.fill == "N-Gon"
                ),
            }
        end,
        inputs = {
            P.v3("center", vector(0, 0, 0)),
            P.scalar("radius", { default = 1.0, min = 0.0 }),
            P.scalar_int("num_vertices", { default = 8, min = 3, soft_max = 32 }),
            P.enum("fill", { "None", "N-Gon" }, 0),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        gizmos = { Gz.tweak_point("center") },
        returns = "out_mesh",
    },
    MakeUVSphere = {
        label = "UV Sphere",
        op = function(inputs)
            return {
                out_mesh = Primitives.uv_sphere(
                    inputs.center,
                    inputs.radius,
                    inputs.segments,
                    inputs.rings
                ),
            }
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
        gizmos = { Gz.tweak_point("center") },
        returns = "out_mesh",
    },
    MakeLine = {
        label = "Line",
        op = function(inputs)
            return {
                out_mesh = Primitives.line(inputs.start_point, inputs.end_point, inputs.segments),
            }
        end,
        inputs = {
            P.v3("start_point", vector(0, 0, 0)),
            P.v3("end_point", vector(0.0, 1.0, 0.0)),
            P.scalar_int("segments", { default = 1, min = 1, soft_max = 32 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        gizmos = { Gz.tweak_point("start_point"), Gz.tweak_point("end_point") },
        returns = "out_mesh",
    },
    MakeTerrain = {
        label = "Terrain",
        op = function(inputs)
            local f = Utils.load_function(inputs.code)
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
        label = "Lua String",
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
    MakeComment = {
        label = "Comment",
        op = function(inputs)
            return {}
        end,
        inputs = {
            P.strparam("comment", "", true),
        },
        outputs = {},
    },
    MakePolygon = {
        label = "Polygon",
        op = function(inputs)
            local points = {}
            -- Parse the point list, separated by space
            for point in inputs.points:gmatch("([^ \n]+)") do
                table.insert(points, V.from_string(point))
            end
            return { out_mesh = Primitives.polygon(points) }
        end,
        inputs = {
            P.strparam("points", "", true),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    MakeLineFromPoints = {
        label = "Line From Points",
        op = function(inputs)
            local points = {}
            -- Parse the point list, separated by space
            for point in inputs.points:gmatch("([^ \n]+)") do
                table.insert(points, V.from_string(point))
            end
            return { out_mesh = Primitives.line_from_points(points) }
        end,
        inputs = {
            P.strparam("points", "", true),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    MakeCone = {
        label = "Cone",
        op = function(inputs)
            return {
                out_mesh = Primitives.cone(
                    inputs.center,
                    inputs.bottom_radius,
                    inputs.top_radius,
                    inputs.height,
                    inputs.num_vertices
                ),
            }
        end,
        inputs = {
            P.v3("center", vector(0, 0, 0)),
            P.scalar("bottom_radius", { default = 1.0, min = 0.0 }),
            P.scalar("top_radius", { default = 0.0, min = 0.0 }),
            P.scalar("height", { default = 1.0, min = 0.0 }),
            P.scalar_int("num_vertices", { default = 8, min = 3, soft_max = 32 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        gizmos = { Gz.tweak_point("center") },
        returns = "out_mesh",
    },
    MakeCylinder = {
        label = "Cylinder",
        op = function(inputs)
            return {
                out_mesh = Primitives.cylinder(
                    inputs.center,
                    inputs.radius,
                    inputs.height,
                    inputs.num_vertices
                ),
            }
        end,
        inputs = {
            P.v3("center", vector(0, 0, 0)),
            P.scalar("radius", { default = 1.0, min = 0.0 }),
            P.scalar("height", { default = 1.0, min = 0.0 }),
            P.scalar_int("num_vertices", { default = 8, min = 3, soft_max = 32 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        gizmos = { Gz.tweak_point("center") },
        returns = "out_mesh",
    },
    MakeGrid = {
        label = "Point Grid",
        op = function(inputs)
            return {
                out_mesh = Primitives.grid(inputs.x, inputs.y, inputs.spacing_x, inputs.spacing_y),
            }
        end,
        inputs = {
            P.scalar_int("x", { default = 3, min = 1, soft_max = 32 }),
            P.scalar_int("y", { default = 3, min = 1, soft_max = 32 }),
            P.scalar("spacing_x", { default = 1.0, min = 0.0 }),
            P.scalar("spacing_y", { default = 1.0, min = 0.0 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
    },
    MakeCatenary = {
        label = "Catenary",
        op = function(inputs)
            return {
                out_mesh = Primitives.catenary(
                    inputs.start_point,
                    inputs.end_point,
                    inputs.sag,
                    inputs.segments
                ),
            }
        end,
        inputs = {
            P.v3("start_point", vector(0, 0, 0)),
            P.v3("end_point", vector(1, 0, 0)),
            P.scalar("sag", { default = 1.0, min = 0.001 }),
            P.scalar_int("segments", { default = 8, min = 1, soft_max = 32 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        gizmos = { Gz.tweak_point("start_point"), Gz.tweak_point("end_point") },
        returns = "out_mesh",
    },
    MakeIcosahedron = {
        label = "Icosahedron",
        op = function(inputs)
            return {
                out_mesh = Primitives.icosahedron(inputs.center, inputs.radius)
            }
        end,
        inputs = {
            P.v3("center", vector(0, 0, 0)),
            P.scalar("radius", {default = 1.0, min = 0}),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        gizmos = { Gz.tweak_point("center"), },
        returns = "out_mesh",
    },
}

-- Edit ops: Nodes to edit existing meshes
local edit_ops = {
    BevelEdges = {
        label = "Bevel Edges",
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
        label = "Chamfer Vertices",
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
        label = "Extrude Faces",
        inputs = {
            P.mesh("in_mesh"),
            P.selection("faces"),
            P.scalar("amount", { default = 0.0 }),
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
    ExtrudeFacesWithCaps = {
        label = "Extrude Faces With Caps",
        inputs = {
            P.mesh("in_mesh"),
            P.selection("faces"),
            P.scalar("amount", { default = 0.0 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.extrude_with_caps(inputs.faces, inputs.amount, out_mesh)
            return { out_mesh = out_mesh }
        end,
    },
    CollapseEdge = {
        label = "Collapse Edges",
        inputs = {
            P.mesh("in_mesh"),
            P.selection("edges"),
            P.scalar("interp", { default = 0.5, soft_min = 0.0, soft_max = 1.0 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.collapse_edge(out_mesh, inputs.edges, inputs.interp)
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
        label = "Make Quad",
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
        label = "Merge Meshes",
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
                    out_mesh = Ops.subdivide(
                        inputs.mesh,
                        inputs.iterations,
                        inputs.technique == "catmull-clark"
                    ),
                }
            end
        end,
    },
    SubdivideEdge = {
        label = "Divide Edges",
        inputs = {
            P.mesh("in_mesh"),
            P.selection("edges"),
            P.scalar("interp", { default = 0.5, soft_min = 0.0, soft_max = 1.0 }),
            P.scalar_int("divisions", { default = 1, min = 1, soft_max = 32 }),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.divide_edges(out_mesh, inputs.edges, inputs.interp, inputs.divisions)
            return { out_mesh = out_mesh }
        end,
    },
    CutFace = {
        label = "Cut Face",
        inputs = {
            P.mesh("in_mesh"),
            P.selection("a"),
            P.selection("b"),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = inputs.in_mesh:clone()
            Ops.cut_face(out_mesh, inputs.a, inputs.b)
            return { out_mesh = out_mesh }
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
            return {
                out_mesh = out_mesh,
            }
        end,
        gizmos = { Gz.tweak_transform("translate", "rotate", "scale") },
    },
    VertexAttribTransfer = {
        label = "Vertex Attribute Transfer",
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
        label = "Set Full Range UVs",
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
        label = "Set Material",
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
        label = "Group",
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
            local typ = Utils.parse_ch_key(inputs.type)
            Ops.make_group(out_mesh, typ, inputs.selection, inputs.name)
            return { out_mesh = out_mesh }
        end,
    },
    EditChannels = {
        label = "Edit Channels",
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
            local k_typ = Utils.parse_ch_key(inputs.channel_key)

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
                local val_typ = Utils.parse_ch_val(ch_val_str)
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
        label = "Copy To Points",
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
        label = "Extrude Along Curve",
        op = function(inputs)
            return {
                out_mesh = Ops.extrude_along_curve(
                    inputs.backbone,
                    inputs.cross_section,
                    inputs.flip
                ),
            }
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
        label = "Resample Curve",
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
        label = "Point Cloud",
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
        label = "Randomize Size",
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
    EditGeometry = {
        label = "Edit Geometry",
        op = function(inputs)
            local out_mesh = inputs.mesh:clone()

            -- Gizmo computation: Compute the midpoint of the group of vertices
            -- being edited. This will be use to compute the gizmo pre-transform.
            if inputs.__gizmos_enabled ~= nil then
                local vertices = {}
                if inputs.geometry == "Vertex" then
                    vertices = out_mesh:resolve_vertex_selection_full(inputs.selection)
                elseif inputs.geometry == "Face" then
                    for _, face in out_mesh:resolve_face_selection_full(inputs.selection) do
                        T.concat(vertices, out_mesh:face_vertices(face))
                    end
                elseif inputs.geometry == "Halfedge" then
                    for _, edge in out_mesh:resolve_halfedge_selection_full(inputs.selection) do
                        local x, y = out_mesh:halfedge_vertices(edge)
                        table.insert(vertices, x)
                        table.insert(vertices, y)
                    end
                end

                local midpoint = vector(0, 0, 0)
                local npoints = 0
                for _, vertex in vertices do
                    midpoint = midpoint + out_mesh:vertex_position(vertex)
                    npoints = npoints + 1
                end
                inputs.gizmo_midpoint = midpoint / npoints
            end

            -- Call the actual op
            local kty = Utils.parse_ch_key(inputs.geometry)
            Ops.edit_geometry(
                out_mesh,
                kty,
                inputs.selection,
                inputs.translate,
                inputs.rotate,
                inputs.scale
            )

            return { out_mesh = out_mesh }
        end,
        returns = "out_mesh",
        inputs = {
            P.mesh("mesh"),
            P.enum("geometry", { "Vertex", "Face", "Halfedge" }),
            P.selection("selection"),
            P.v3("translate", vector(0, 0, 0)),
            P.v3("rotate", vector(0, 0, 0)),
            P.v3("scale", vector(1, 1, 1)),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        gizmos = {
            Gz.tweak_transform(
                "translate",
                "rotate",
                "scale",
                { pre_translation_param = "gizmo_midpoint" }
            ),
        },
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
        label = "Vector",
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
        label = "Vector Math",
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
        label = "Scalar Math",
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
            P.file("path", "open"),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local out_mesh = HalfEdgeMesh.from_wavefront_obj(inputs.path)
            return { out_mesh = out_mesh }
        end,
    },
}

-- Miscelaneous nodes
local misc = {
    -- A point, returning a single vector shows a tweakable gizmo
    Point = {
        label = "Point",
        inputs = {
            P.v3("point", vector(0, 0, 0)),
        },
        outputs = {
            P.v3("point"),
        },
        op = function(inputs)
            return { point = inputs.point }
        end,
        gizmos = { Gz.tweak_point("point") },
    },
    Turntable = {
        label = "Turntable",
        doc = [[
            Will rotate the current mesh over time, centered at its origin.
            This rotation is not exported to the end mesh, but is helpful
            when you want to show off your creations in blackjack itself.
        ]],
        inputs = {
            P.scalar("speed", { default = 1.0, min = 0.0 }),
            P.mesh("mesh"),
        },
        outputs = {
            P.mesh("out_mesh"),
        },
        returns = "out_mesh",
        op = function(inputs)
            local time = os.clock()
            local out_mesh = inputs.mesh:clone()
            Ops.transform(
                out_mesh,
                vector(0, 0, 0),
                vector(0, time * inputs.speed, 0),
                vector(1, 1, 1)
            )
            return {
                out_mesh = out_mesh,
            }
        end,
    },
}

NodeLibrary:addNodes(primitives)
NodeLibrary:addNodes(edit_ops)
NodeLibrary:addNodes(math_nodes)
NodeLibrary:addNodes(export)
NodeLibrary:addNodes(misc)
