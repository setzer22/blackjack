use egui_node_graph::{InputId, NodeId, OutputId};
use mlua::{ExternalResult, Lua, ToLua};

use crate::prelude::*;

use std::fmt::Write;

use super::node_graph::Graph;

/// Represents the place where the output of a node will be stored. Gets
/// translated to local identifiers in the Lua code.
///
/// The truct itself only stores the necessary data to generate the identifiers,
/// not the strings. This makes it cheap to copy.
#[derive(Clone, Copy)]
struct NodeOutputAddr {
    /// The id for the node that produced this output. The outputs are stored in
    /// a table with names being the names of the output parameters of this
    /// node, so they are implicit.
    id: NodeId,
}

/// Represents the place where a constant parameter is stored. Gets translated
/// to local identifiers in the Lua code, referencing the `input_params_sym`
/// table in the context.

/// The truct itself only stores the necessary data to generate the identifiers,
/// not the strings. This makes it cheap to copy.
#[derive(Clone, Copy)]
pub struct ConstParamAddr {
    /// The id for the node input that contains the constant. The value itself
    /// is not generated here, but in the constant extraction phase. This allows
    /// compiling a Lua program once and running it multiple times with
    /// different constant sets without recompilation.
    pub id: InputId,
}

/// When passing inputs to nodes, those can be outputs from another node (the
/// `OtherOut` variant) or constant parameters (the `ConstParam` variant)
enum InputArgAddr {
    OtherOut {
        out_addr: NodeOutputAddr,
        output_id: OutputId,
    },
    ConstParam(ConstParamAddr),
}

/// A context object for code generation, storing several fields
struct CodegenContext {
    /// The identifier for the single argument to the generated function. This
    /// argument is going to be a table, with one field per constant parameter.
    input_params_ident: String,
    /// The program string. Code generation modifies this value by appending
    /// lines to it.
    lua_program: String,
    /// When an input reads a value from another graph's output, this output is
    /// cached here so that other nodes reading that output can reference the
    /// same variable in the code.
    outputs_cache: HashMap<OutputId, NodeOutputAddr>,
    /// Every time a const parameter address is generated, the address is pushed
    /// into this vector.
    const_parameters: Vec<ConstParamAddr>,
    /// The current indent level. Gets added / subtracted as we nesting levels
    /// increase. Each indent level equals four spaces.
    indent_level: usize,
}

/// The resulting compiled program
pub struct CompiledProgram {
    /// A string of lua code, ready to be loaded by the Lua runtime.
    pub lua_program: String,
    /// The list of constant parameter addresses extracted from the graph when
    /// generating this program. This tells the constant extractor the constant
    /// values it needs to find in the graph. Constants are the widgets inside
    /// the nodes that appear for some data types when there's no input
    /// connection.
    pub const_parameters: Vec<ConstParamAddr>,
}

/// Returns a string uniquely idenfifying a slotmap id. This will be an
/// identifier like `1v1` for index 1, generation 1, but the value is really an
/// implementation detail.
///
/// NOTE: There is a potential for updates in the `slotmap` crate breaking our
/// code generator if their debug representation starts following a different
/// pattern that's no longer compatible with Lua identifier syntax.
pub fn slotmap_id_str<K: slotmap::Key>(id: K) -> String {
    format!("{:?}", id.data())
}

impl NodeOutputAddr {
    /// The string that should be used to generate the variable name where the
    /// node output will be stored. For instance `MakeBox_1v1`.
    fn variable_name(self, graph: &Graph) -> Result<String> {
        let node = &graph[self.id];
        let unique = slotmap_id_str(node.id);
        Ok(format!("{}_{}", node.user_data.op_name, unique,))
    }

    /// The string that should be used to reference an output value from this
    /// node. For instance: `MakeBox_1v1.out_mesh`
    fn output_value_ref(self, graph: &Graph, output_id: OutputId) -> Result<String> {
        let node = &graph[self.id];
        let unique = slotmap_id_str(node.id);
        let param_name = node
            .outputs
            .iter()
            .find(|(_, x)| *x == output_id)
            .map(|(x, _)| x)
            .ok_or_else(|| anyhow!("Error creating string ident"))?;
        Ok(format!(
            "{}_{}.{}",
            node.user_data.op_name, unique, param_name
        ))
    }
}

impl ConstParamAddr {
    /// The string that should be used to reference an input value from the
    /// program constants, something like `input_params.MakeBox_1v1_size`.
    pub fn const_value_ref(self, graph: &Graph) -> Result<String> {
        let param = &graph[self.id];
        let node = &graph[param.node];
        let op_name = &node.user_data.op_name;
        let unique = slotmap_id_str(node.id);
        let param_name = node
            .inputs
            .iter()
            .find(|(_, x)| *x == self.id)
            .map(|(x, _)| x)
            .ok_or_else(|| anyhow!("Error creating string ident"))?;
        Ok(format!("{op_name}_{unique}_{param_name}",))
    }
}

impl InputArgAddr {
    /// Returns the string that should be used to reference this value,
    /// depending on whether it's a constant or another node's output.
    fn generate_code(self, graph: &Graph, ctx: &mut CodegenContext) -> Result<String> {
        match self {
            InputArgAddr::OtherOut {
                out_addr: out,
                output_id: param,
            } => out.output_value_ref(graph, param),
            InputArgAddr::ConstParam(param_sym) => {
                let input_params_ident = &ctx.input_params_ident;
                let ident = param_sym.const_value_ref(graph)?;
                Ok(format!("{input_params_ident}.{ident}",))
            }
        }
    }
}

/// Generates code for a graph node's input.
fn codegen_input(
    graph: &Graph,
    ctx: &mut CodegenContext,
    node_id: NodeId,
    param_name: &str,
) -> Result<InputArgAddr> {
    let param = graph[node_id].get_input(param_name)?;
    if let Some(output) = graph.connection(param) {
        if let Some(ident) = ctx.outputs_cache.get(&output) {
            Ok(InputArgAddr::OtherOut {
                out_addr: *ident,
                output_id: output,
            })
        } else {
            codegen_node(graph, ctx, graph[output].node, false)?;
            Ok(InputArgAddr::OtherOut {
                out_addr: *ctx
                    .outputs_cache
                    .get(&output)
                    .expect("Codegen should populate the cache"),
                output_id: output,
            })
        }
    } else {
        match graph[param].value().0 {
            ValueType::None => Err(anyhow!(
                "Parameter {} of node {:?} should have a connection",
                param_name,
                node_id
            )),
            ValueType::Vector(_)
            | ValueType::Scalar { .. }
            | ValueType::Selection { .. }
            | ValueType::Enum { .. }
            | ValueType::NewFile { .. }
            | ValueType::String { .. } => {
                let addr = ConstParamAddr { id: param };
                ctx.const_parameters.push(addr);
                Ok(InputArgAddr::ConstParam(addr))
            }
        }
    }
}

/// Generates code for a graph node's output.
fn codegen_output(
    graph: &Graph,
    ctx: &mut CodegenContext,
    node_id: NodeId,
) -> Result<NodeOutputAddr> {
    let addr = NodeOutputAddr { id: node_id };
    for (_, out_id) in graph[node_id].outputs.iter() {
        ctx.outputs_cache.insert(*out_id, addr);
    }
    Ok(addr)
}

/// Generates the code for a node, ensuring all the code to produce its inputs
/// is recursively generated, and storing the addresses for its outputs on the
/// outputs cache.
fn codegen_node(
    graph: &Graph,
    ctx: &mut CodegenContext,
    node_id: NodeId,
    target: bool,
) -> Result<()> {
    let indent = "    ".repeat(ctx.indent_level);

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
    let args = if graph[node_id].inputs.is_empty() {
        String::from("{}")
    } else {
        let mut args = String::from("{\n");
        for input_name in graph[node_id].inputs.iter().map(|x| &x.0) {
            let input_addr = codegen_input(graph, ctx, node_id, input_name)?;
            writeln!(
                args,
                "{indent}{indent}{input_name} = {},",
                input_addr.generate_code(graph, ctx)?
            )?;
        }
        args + indent.as_str() + "}"
    };
    let output_addr = codegen_output(graph, ctx, node_id)?.variable_name(graph)?;
    let node_name = graph[node_id].user_data.op_name.as_str();

    emit_line!("local {output_addr} = NodeLibrary:callNode('{node_name}', {args})");

    // TODO: The return value is not always out_mesh. This should be stored
    // somehow in the node definition.
    emit_return!(format!("{output_addr}.out_mesh"));

    Ok(())
}

/// Compiles a graph into a Lua program. The program produced computes and
/// returns the value of the `final_node`.
pub fn compile_graph(graph: &Graph, final_node: NodeId) -> Result<CompiledProgram> {
    let input_params_ident = "input_params";
    let mut ctx = CodegenContext {
        indent_level: 1,
        input_params_ident: input_params_ident.into(),
        lua_program: String::new(),
        outputs_cache: Default::default(),
        const_parameters: Default::default(),
    };

    writeln!(ctx.lua_program, "function main({input_params_ident})")?;
    codegen_node(graph, &mut ctx, final_node, true)?;
    writeln!(ctx.lua_program, "end")?;
    Ok(CompiledProgram {
        lua_program: ctx.lua_program,
        const_parameters: ctx.const_parameters,
    })
}

/// Extracts parameters from a graph into a Lua table
pub fn extract_params<'lua>(
    lua: &'lua Lua,
    graph: &Graph,
    compiled: &CompiledProgram,
) -> Result<mlua::Table<'lua>> {
    let table = lua.create_table()?;
    for const_param in &compiled.const_parameters {
        let id = const_param.id;
        let input = graph.get_input(id);
        let ident = const_param.const_value_ref(graph)?;
        let value = match &input.value().0 {
            ValueType::None => {
                Err(anyhow!("Cannot use constant value for non-existing type")).to_lua_err()
            }
            ValueType::Vector(v) => LVec3(*v).to_lua(lua),
            ValueType::Scalar { value, .. } => value.to_lua(lua),
            ValueType::Selection { selection, .. } => selection
                .clone()
                .unwrap_or(SelectionExpression::None)
                .to_lua(lua),
            ValueType::Enum {
                values,
                selected: selection,
            } => values[selection.unwrap_or(0) as usize].clone().to_lua(lua),
            ValueType::NewFile { path } => blackjack_engine::lua_engine::lua_stdlib::Path(
                path.as_ref()
                    .ok_or_else(|| anyhow!("Path not set"))?
                    .clone(),
            )
            .to_lua(lua),
            ValueType::String { text, .. } => text.clone().to_lua(lua),
        }?;
        table.set(ident, value)?;
    }
    Ok(table)
}
