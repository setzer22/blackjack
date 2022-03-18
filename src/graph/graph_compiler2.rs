use crate::{graph::poly_asm::PolyAsmProgram, prelude::graph::*, prelude::*};

use super::poly_asm::{MemAddr, PolyAsmInstruction};
use std::fmt::Write;

#[derive(Clone, Copy)]
struct OutSym {
    id: OutputId,
    counter: usize,
}

#[derive(Clone, Copy)]
pub struct ParamSym {
    pub id: InputId,
    pub counter: usize,
}

enum Sym {
    OtherOut(OutSym),
    Param(ParamSym),
}

#[derive(Default)]
pub struct SymGenerator {
    counter: usize,
}
impl SymGenerator {
    fn output_id_gen(&mut self, id: OutputId) -> OutSym {
        self.counter += 1;
        OutSym {
            id,
            counter: self.counter,
        }
    }

    fn param_sym_gen(&mut self, id: InputId) -> ParamSym {
        self.counter += 1;
        ParamSym {
            id,
            counter: self.counter,
        }
    }
}

impl OutSym {
    fn ident_str(self, graph: &Graph) -> Result<String> {
        let counter = self.counter;
        let param = &graph[self.id];
        let node = &graph[param.node];
        let param_name = node
            .outputs
            .iter()
            .find(|(_, x)| *x == self.id)
            .map(|(x, _)| x)
            .ok_or_else(|| anyhow!("Error creating string ident"))?;
        Ok(format!(
            "{}_{}_{}",
            node.user_data.op_name, param_name, counter
        ))
    }
}

impl ParamSym {
    pub fn ident_str(self, graph: &Graph) -> Result<String> {
        let param = &graph[self.id];
        let node = &graph[param.node];
        let op_name = &node.user_data.op_name;
        let counter = self.counter;
        let param_name = node
            .inputs
            .iter()
            .find(|(_, x)| *x == self.id)
            .map(|(x, _)| x)
            .ok_or_else(|| anyhow!("Error creating string ident"))?;
        Ok(format!("{op_name}_{param_name}_{counter}",))
    }
}

impl Sym {
    fn generate_code(self, graph: &Graph, ctx: &mut CodegenContext) -> Result<String> {
        match self {
            Sym::OtherOut(out_sym) => out_sym.ident_str(graph),
            Sym::Param(param_sym) => {
                let input_params_sym = &ctx.input_params_sym;
                let ident = param_sym.ident_str(graph)?;
                Ok(format!("{input_params_sym}.{ident}",))
            }
        }
    }
}

struct CodegenContext {
    input_params_sym: String,
    lua_program: String,
    outputs_cache: HashMap<OutputId, OutSym>,
    const_parameters: Vec<ParamSym>,
    sym_gen: SymGenerator,
    indent_level: usize,
}

pub struct CompiledProgram {
    pub lua_program: String,
    pub const_parameters: Vec<ParamSym>,
}

fn codegen_input(
    graph: &Graph,
    ctx: &mut CodegenContext,
    node_id: NodeId,
    param_name: &str,
) -> Result<Sym> {
    let param = graph[node_id].get_input(param_name)?;
    if let Some(output) = graph.connection(param) {
        if let Some(ident) = ctx.outputs_cache.get(&output) {
            Ok(Sym::OtherOut(*ident))
        } else {
            codegen_node(graph, ctx, graph[output].node, false)?;
            Ok(Sym::OtherOut(
                *ctx.outputs_cache
                    .get(&output)
                    .expect("Codegen should populate the cache"),
            ))
        }
    } else {
        match graph[param].value() {
            ValueType::None => Err(anyhow!(
                "Parameter {} of node {:?} should have a connection",
                param_name,
                node_id
            )),
            ValueType::Vector(_)
            | ValueType::Scalar { .. }
            | ValueType::Selection { .. }
            | ValueType::Enum { .. }
            | ValueType::NewFile { .. } => {
                let sym = ctx.sym_gen.param_sym_gen(param);
                ctx.const_parameters.push(sym);
                Ok(Sym::Param(sym))
            }
        }
    }
}

fn codegen_output(
    graph: &Graph,
    ctx: &mut CodegenContext,
    node_id: NodeId,
    param_name: &str,
) -> Result<OutSym> {
    let out_id = graph[node_id].get_output(param_name)?;
    let sym = ctx.sym_gen.output_id_gen(out_id);
    ctx.outputs_cache.insert(out_id, sym);
    Ok(sym)
}

/// Generates the code for a node, ensuring all the code to produce its inputs
/// is recursively generated, and storing the addresses for its outputs on the
/// outputs cache.
fn codegen_node(graph: &Graph, ctx: &mut CodegenContext, node_id: NodeId, target: bool) -> Result<()> {
    let indent = "    ".repeat(ctx.indent_level);

    macro_rules! input {
        ($name:expr) => {
            codegen_input(graph, ctx, node_id, $name)?.generate_code(graph, ctx)?
        };
    }
    macro_rules! output {
        ($name:expr) => {
            Sym::OtherOut(codegen_output(graph, ctx, node_id, $name)?).generate_code(graph, ctx)?
        };
    }
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
        }
    }

    match graph[node_id].user_data.op_name.as_str() {
        "MakeBox" => {
            let origin = input!("origin");
            let size = input!("size");
            let mesh = output!("out_mesh");
            emit_line!("local {mesh} = Primitives.cube({origin}, {size});");
            emit_return!(mesh);
        }
        "MakeQuad" => {
            let center = input!("center");
            let normal = input!("normal");
            let right = input!("right");
            let size = input!("size");
            let mesh = output!("out_mesh");
            emit_line!("local {mesh} = Primitives.quad({center}, {normal}, {right}, {size});");
            emit_return!(mesh);
        }
        "BevelEdges" => {
            let edges = input!("edges");
            let amount = input!("amount");
            let in_mesh = input!("in_mesh");
            let out_mesh = output!("out_mesh");
            emit_line!("local {out_mesh} = Ops.bevel({edges}, {amount}, {in_mesh});");
            emit_return!(out_mesh);
        }
        invalid => return Err(anyhow!("Unknown op_name {}", invalid)),
    }
    Ok(())
}

pub fn compile_graph(graph: &Graph, final_node: NodeId) -> Result<CompiledProgram> {
    let input_params_sym = "input_params";
    let mut ctx = CodegenContext {
        indent_level: 1,
        input_params_sym: input_params_sym.into(),
        lua_program: String::new(),
        outputs_cache: Default::default(),
        const_parameters: Default::default(),
        sym_gen: Default::default(),
    };

    writeln!(ctx.lua_program, "function main({input_params_sym})")?;
    codegen_node(graph, &mut ctx, final_node, true)?;
    writeln!(ctx.lua_program, "end")?;
    Ok(CompiledProgram {
        lua_program: ctx.lua_program,
        const_parameters: ctx.const_parameters,
    })
}
