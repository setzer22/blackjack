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

pub fn run_example(rt: &LuaRuntime, bjk_data: &str) -> Result<ProgramResult> {
    let (rt_data, _, _) = SerializedBjkGraph::load_from_string(bjk_data)?.into_runtime()?;
    run_graph(
        &rt.lua,
        &rt_data.graph,
        infer_target_node(&rt_data.graph),
        rt_data.external_parameters.unwrap(),
        &rt.node_definitions,
        None,
    )
}

#[test]
pub fn test_box() -> Result<()> {
    let lua_runtime = LuaRuntime::initialize_with_std("../blackjack_lua".into())?;
    let result = run_example(&lua_runtime, include_str!("../../examples/Box.bjk"))?;
    if let Some(RenderableThing::HalfEdgeMesh(h)) = result.renderable {
        assert_eq!(h.read_connectivity().num_vertices(), 8);
        assert_eq!(h.read_connectivity().num_halfedges(), 24);
        assert_eq!(h.read_connectivity().num_faces(), 6);
    } else {
        panic!("Expected a mesh")
    }
    Ok(())
}
