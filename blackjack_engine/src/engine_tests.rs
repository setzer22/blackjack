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
    ).unwrap()
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
        },
        Example {
            path: "../examples/tp_cutter.bjk",
            vertices: 184,
            halfedges: 680,
            faces: 170,
        },
        Example {
            path: "../examples/stylised_sword.bjk",
            vertices: 284,
            halfedges: 988,
            faces: 228,
        },
    ];

    for example in examples {
        println!("Loading example at {}", example.path);
        let result = run_example(&example, &lua_runtime);
        if let Some(RenderableThing::HalfEdgeMesh(h)) = result.renderable {
            assert_eq!(h.read_connectivity().num_vertices(), example.vertices);
            assert_eq!(h.read_connectivity().num_halfedges(), example.halfedges);
            assert_eq!(h.read_connectivity().num_faces(), example.faces);
        } else {
            panic!("Expected a mesh")
        }
    }
}
