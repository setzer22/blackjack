use crate::graph::*;
use crate::prelude::*;
use derive_more::Deref;
use derive_more::DerefMut;
use derive_more::Display;
use mlua::ToLua;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Write;

/// The Lua symbol representing a key in the external inputs dictionary
#[derive(Debug, Clone, Display, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ExternalParamAddr(pub String);

/// The Lua symbol to representing a dictionary containing a node's outputs
#[derive(Debug, Clone, Display)]
pub struct NodeOutputAddr(pub String);

/// External parameters can be provided to the graph from the outside. They
/// correspond to all input properties not connected to any node.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalParameterDef {
    pub addr: ExternalParamAddr,
    pub data_type: DataType,
    pub node_id: BjkNodeId,
    pub param_name: String,
}

impl ExternalParameterDef {
    pub fn new(graph: &BjkGraph, node_id: BjkNodeId, param: &str) -> Result<Self> {
        let node = &graph.nodes[node_id];
        let input = node
            .inputs
            .iter()
            .find(|input| input.name == param)
            .ok_or_else(|| anyhow!("Param not found: {param}"))?;
        let op_name = &node.op_name;
        Ok(ExternalParameterDef {
            addr: ExternalParamAddr(format!("{op_name}_{}_{param}", node_id.display_id())),
            data_type: input.data_type,
            node_id,
            param_name: param.into(),
        })
    }
}

#[derive(Default, Debug, Clone, Deref, DerefMut, Serialize, Deserialize)]
pub struct ExternalParameterValues(HashMap<ExternalParamAddr, BlackjackParameter>);
impl ExternalParameterValues {
    pub fn make_input_table<'lua>(&self, lua: &'lua mlua::Lua) -> Result<mlua::Table<'lua>> {
        let table = lua.create_table()?;
        for (k, v) in &self.0 {
            table.set(k.clone().0.to_lua(lua)?, v.clone().value.to_lua(lua)?)?;
        }
        Ok(table)
    }
}

/// The resulting compiled program
#[derive(Debug, Serialize, Deserialize)]
pub struct CompiledProgram {
    /// A string of lua code, ready to be loaded by the Lua runtime.
    pub lua_program: String,
    /// The list of constant parameter addresses extracted from the graph when
    /// generating this program. This tells the constant extractor the constant
    /// values it needs to find in the graph. Constants are the widgets inside
    /// the nodes that appear for some data types when there's no input
    /// connection.
    pub external_parameters: Vec<ExternalParameterDef>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlackjackGameAsset {
    pub program: CompiledProgram,
    pub params: ExternalParameterValues,
}

/// A context object for code generation, storing several fields
struct CodegenContext {
    /// The single table argument to the generated function. Stores the external
    /// parameters.
    input_params_ident: String,
    /// The program string. Code generation modifies this value by appending
    /// lines to it.
    lua_program: String,
    /// When an input reads a value from another graph's output, the Lua symbol
    /// representing this output is cached here so that other nodes reading that
    /// output can reference the same variable in the code.
    outputs_cache: HashMap<BjkNodeId, NodeOutputAddr>,
    /// Every time an external parameter is referenced in the generated code,
    /// its definition is pushed into this vector.
    external_parameters: Vec<ExternalParameterDef>,
    /// The length of an indent, in spaces.
    indent_length: usize,
}

pub fn compile_graph(graph: &BjkGraph, final_node: BjkNodeId) -> Result<CompiledProgram> {
    let input_params_ident = "input_params";
    let mut ctx = CodegenContext {
        indent_length: 4,
        input_params_ident: input_params_ident.into(),
        lua_program: String::new(),
        outputs_cache: Default::default(),
        external_parameters: Default::default(),
    };

    writeln!(ctx.lua_program, "function main({input_params_ident})")?;
    codegen_node(graph, &mut ctx, final_node, true)?;
    writeln!(ctx.lua_program, "end")?;
    Ok(CompiledProgram {
        lua_program: ctx.lua_program,
        external_parameters: ctx.external_parameters,
    })
}

fn codegen_node(
    graph: &BjkGraph,
    ctx: &mut CodegenContext,
    node_id: BjkNodeId,
    target: bool,
) -> Result<()> {
    let indent = " ".repeat(ctx.indent_length);

    macro_rules! emit_line {
        ($($exprs:expr),*) => {
            write!(ctx.lua_program, "{indent}")?;
            writeln!(
                ctx.lua_program,
                $($exprs),*
            )?;
        }
    }
    macro_rules! emit_return {
        ($name:expr) => {
            if target {
                emit_line!("return {};", $name);
            }
        };
    }

    let node = &graph.nodes[node_id];

    // Generate code for dependent nodes and populate the output cache
    for input in &node.inputs {
        match &input.kind {
            DependencyKind::Connection {
                node: other_node, ..
            } => {
                if !ctx.outputs_cache.contains_key(other_node) {
                    codegen_node(graph, ctx, *other_node, false)?;
                }
            }
            DependencyKind::Computed(_) | DependencyKind::External => {}
        }
    }

    let args = if node.inputs.is_empty() {
        String::from("{}")
    } else {
        let mut args = String::from("{\n");
        for input in &node.inputs {
            //let input_addr = codegen_input(graph, ctx, node_id, input_name)?;
            let input_name = &input.name;

            let input_value = match &input.kind {
                DependencyKind::Computed(lua_code) => {
                    // A lua expression will be copied verbatim
                    // TODO: Maybe we want to enclose this in a function?
                    lua_code.0.clone()
                }
                DependencyKind::External => {
                    // External variables fome from the outside
                    let input_params_ident = &ctx.input_params_ident;
                    let external_param = ExternalParameterDef::new(graph, node_id, &input.name)?;
                    let addr = external_param.addr.0.clone();
                    ctx.external_parameters.push(external_param);
                    format!("{input_params_ident}.{addr}")
                }
                DependencyKind::Connection { node, param_name } => {
                    // Since we ran code generation for all our dependent
                    // connections above, the output addr is guaranteed to have
                    // been generated by now.
                    let output_addr = ctx
                        .outputs_cache
                        .get(node)
                        .expect("Should've been generated above");
                    format!("{output_addr}.{param_name}")
                }
            };

            writeln!(args, "{indent}{indent}{input_name} = {input_value},",)?;
        }
        args + indent.as_str() + "}"
    };
    let op_name = &node.op_name;
    let output_addr = NodeOutputAddr(format!("{op_name}_{}_out", node_id.display_id()));
    ctx.outputs_cache.insert(node_id, output_addr.clone());

    let node_name = &node.op_name;

    emit_line!("local {output_addr} = NodeLibrary:callNode('{node_name}', {args})");

    // TODO: The return value is not always out_mesh. This should be stored
    // somehow in the node definition.
    emit_return!(format!("{output_addr}.out_mesh"));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    pub fn compile_simple_graph_test() {
        let mut graph = BjkGraph::new();

        let cube = graph.add_node("Box");
        graph.add_input(cube, "origin", DataType::Vector).unwrap();
        graph.add_input(cube, "size", DataType::Scalar).unwrap();
        graph.add_output(cube, "out_mesh", DataType::Mesh).unwrap();

        let transform = graph.add_node("Translate");
        graph.add_input(transform, "mesh", DataType::Mesh).unwrap();
        graph
            .add_input(transform, "translate", DataType::Vector)
            .unwrap();
        graph
            .add_output(transform, "out_mesh", DataType::Mesh)
            .unwrap();

        graph
            .add_connection(cube, "out_mesh", transform, "mesh")
            .unwrap();

        let program = compile_graph(&graph, transform).unwrap();

        let expected_output = r#"function main(input_params)
    local Box_1v1_out = NodeLibrary:callNode('Box', {
        origin = input_params.Box_1v1_origin,
        size = input_params.Box_1v1_size,
    })
    local Translate_2v1_out = NodeLibrary:callNode('Translate', {
        mesh = Box_1v1_out.out_mesh,
        translate = input_params.Translate_2v1_translate,
    })
    return Translate_2v1_out.out_mesh;
end
"#;

        assert_eq!(program.lua_program, expected_output);
    }
}
