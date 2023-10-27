// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::graph::serialization::SerializedBjkGraph;
use crate::graph::{BjkGraph, BjkNodeId};
use crate::graph_interpreter::run_graph;
use crate::lua_engine::{LuaRuntime, ProgramResult, RenderableThing};
use crate::prelude::*;

/// Looks for the first node with no outgoing parameters and assumes it to be
/// the graph's final node. Comment nodes are ignored because examples typically
/// contain those.
pub fn infer_target_node(graph: &BjkGraph) -> BjkNodeId {
    // A set of all nodes which are dependencies to other nodes
    let mut dependencies = HashSet::new();
    for (_, node) in &graph.nodes {
        for input in &node.inputs {
            if let crate::graph::DependencyKind::Connection { node: other, .. } = &input.kind {
                dependencies.insert(other);
            }
        }
    }
    for (node_id, node) in &graph.nodes {
        if !dependencies.contains(&node_id) && node.op_name != "MakeComment" {
            return node_id;
        }
    }
    panic!("Target node heuristic failed")
}

#[derive(Clone, Copy)]
struct Example {
    path: &'static str,
    vertices: usize,
    halfedges: usize,
    faces: usize,
    bounding_box_center: Vec3,
    bounding_box_size: Vec3,
}

fn run_example(example: &Example, rt: &LuaRuntime) -> ProgramResult {
    let bjk_data = std::fs::read_to_string(example.path).unwrap();
    let (rt_data, _, _) = SerializedBjkGraph::load_from_string(&bjk_data)
        .unwrap()
        .into_runtime()
        .unwrap();
    run_graph(
        &rt.lua,
        &rt_data.graph,
        infer_target_node(&rt_data.graph),
        rt_data.external_parameters.unwrap(),
        &rt.node_definitions,
        None,
    )
    .unwrap()
}

#[test]
pub fn test_examples_folder() {
    let lua_runtime = LuaRuntime::initialize_with_std("../blackjack_lua".into()).unwrap();

    let examples = &[
        Example {
            path: "../examples/box.bjk",
            vertices: 8,
            halfedges: 24,
            faces: 6,
            bounding_box_center: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            bounding_box_size: Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        },
        Example {
            path: "../examples/tp_cutter.bjk",
            vertices: 184,
            halfedges: 680,
            faces: 170,
            bounding_box_center: Vec3 {
                x: 0.0,
                y: 0.29355407,
                z: 0.0,
            },
            bounding_box_size: Vec3 {
                x: 1.1,
                y: 0.6210451,
                z: 0.9,
            },
        },
        Example {
            path: "../examples/static-sword.bjk",
            vertices: 284,
            halfedges: 988,
            faces: 228,
            bounding_box_center: Vec3 {
                x: 0.020206064,
                y: 0.14655268,
                z: 0.074326545,
            },
            bounding_box_size: Vec3 {
                x: 1.1485293,
                y: 2.532574,
                z: 1.3417027,
            },
        },
        Example {
            path: "../examples/extrude-quad-along-helix.bjk",
            vertices: 148,
            halfedges: 584,
            faces: 144,
            bounding_box_center: Vec3 {
                x: 0.0,
                y: 1.5,
                z: 0.0,
            },
            bounding_box_size: Vec3 {
                x: 7.0,
                y: 4.0,
                z: 7.0,
            },
        },
    ];

    for example in examples {
        println!("Loading example at {}", example.path);
        let result = run_example(example, &lua_runtime);
        if let Some(RenderableThing::HalfEdgeMesh(h)) = result.renderable {
            assert_eq!(h.read_connectivity().num_vertices(), example.vertices);
            assert_eq!(h.read_connectivity().num_halfedges(), example.halfedges);
            assert_eq!(h.read_connectivity().num_faces(), example.faces);
            let (got_center, got_size) = h.bounding_box();
            assert_eq!(got_center, example.bounding_box_center);
            assert_eq!(got_size, example.bounding_box_size);
        } else {
            panic!("Expected a mesh")
        }
    }
}
