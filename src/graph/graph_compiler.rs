use crate::{graph::poly_asm::PolyAsmProgram, prelude::graph::*, prelude::*};

use self::outputs_cache::OutputsCache;

use super::poly_asm::{MemAddr, PolyAsmInstruction};

mod outputs_cache;

/// Generates code to ensure the input value is computed. May generate code for
/// other nodes recursively.
fn gen_input_value<T>(
    program: &mut PolyAsmProgram,
    graph: &Graph,
    outputs_cache: &mut OutputsCache,
    node_id: NodeId,
    param_name: &str,
) -> Result<MemAddr<T>>
where
    T: Send + Sync + 'static + Clone,
{
    let param = graph[node_id].get_input(param_name)?;
    if let Some(output) = graph.connection(param) {
        if let Some(addr) = outputs_cache.get(output) {
            Ok(addr)
        } else {
            gen_code_for_node(program, graph, graph[output].node(), outputs_cache)?;
            Ok(outputs_cache
                .get(output)
                .expect("Codegen should populate the cache"))
        }
    } else {
        let addr = match graph[param].value() {
            InputParamValue::Vector(val) => Ok(program.mem_alloc_raw(val)),
            InputParamValue::Scalar(val) => Ok(program.mem_alloc_raw(val)),
            InputParamValue::Selection { text, selection } => {
                Ok(program.mem_alloc_raw(selection.ok_or_else(|| {
                    anyhow!("Error parsing selection for parameter {:?}", param_name)
                })?))
            }
            InputParamValue::None => Err(anyhow!(
                "Parameter {} of node {:?} should have a connection",
                param_name,
                node_id
            )),
            InputParamValue::Enum { values, selection } => {
                let selection = selection.ok_or_else(|| {
                    anyhow!("No selection has been made for parameter {}", param_name)
                })?;
                let value: String = values.get(selection as usize).cloned().ok_or_else(|| {
                    anyhow!("Invalid selection index for parameter {}", param_name)
                })?;
                Ok(program.mem_alloc_raw(value))
            }
            InputParamValue::NewFile { path } => {
                let path: std::path::PathBuf =
                    path.ok_or_else(|| anyhow!("Path is not set"))?.clone();
                Ok(program.mem_alloc_raw(path))
            }
        }?;
        MemAddr::from_raw_checked(program, addr, param_name)
    }
}

/// Allocates the return address for an output parameter, and registers this
/// fact on the outputs cache to avoid re-generating code for nodes.
fn gen_output_value<T>(
    program: &mut PolyAsmProgram,
    graph: &Graph,
    outputs_cache: &mut OutputsCache,
    node_id: NodeId,
    param_name: &str,
) -> Result<MemAddr<T>>
where
    T: Send + Sync + 'static + Clone,
{
    let addr = program.mem_reserve();
    let param = graph[node_id].get_output(param_name)?;
    outputs_cache.insert(param, addr);
    Ok(addr)
}

/// Generates the code for a node, ensuring all the code to produce its inputs
/// is recursively generated, and storing the addresses for its outputs on the
/// outputs cache.
fn gen_code_for_node(
    program: &mut PolyAsmProgram,
    graph: &Graph,
    node_id: NodeId,
    outputs_cache: &mut OutputsCache,
) -> Result<()> {
    macro_rules! input {
        ($name:expr) => {
            gen_input_value(program, graph, outputs_cache, node_id, $name)?
        };
    }
    macro_rules! output {
        ($name:expr) => {
            gen_output_value(program, graph, outputs_cache, node_id, $name)?
        };
    }

    match graph[node_id].op_name.clone().as_str() {
        "MakeBox" => {
            let operation = PolyAsmInstruction::MakeCube {
                origin: input!("origin"),
                size: input!("size"),
                out_mesh: output!("out_mesh"),
            };
            program.add_operation(operation);
        }
        "MakeQuad" => {
            let operation = PolyAsmInstruction::MakeQuad {
                center: input!("center"),
                normal: input!("normal"),
                right: input!("right"),
                size: input!("size"),
                out_mesh: output!("out_mesh"),
            };
            program.add_operation(operation);
        }
        "BevelEdges" => {
            let operation = PolyAsmInstruction::BevelEdges {
                edges: input!("edges"),
                amount: input!("amount"),
                in_mesh: input!("in_mesh"),
                out_mesh: output!("out_mesh"),
            };
            program.add_operation(operation);
        }
        "ExtrudeFaces" => {
            let operation = PolyAsmInstruction::ExtrudeFaces {
                faces: input!("faces"),
                amount: input!("amount"),
                in_mesh: input!("in_mesh"),
                out_mesh: output!("out_mesh"),
            };
            program.add_operation(operation);
        }
        "ChamferVertices" => {
            let operation = PolyAsmInstruction::ChamferVertices {
                vertices: input!("vertices"),
                amount: input!("amount"),
                in_mesh: input!("in_mesh"),
                out_mesh: output!("out_mesh"),
            };
            program.add_operation(operation);
        }
        "MakeVector" => {
            let operation = PolyAsmInstruction::MakeVector {
                x: input!("x"),
                y: input!("y"),
                z: input!("z"),
                out_vec: output!("out_vec"),
            };
            program.add_operation(operation);
        }
        "VectorMath" => {
            let op: MemAddr<String> = input!("vec_op");
            let op_str = program
                .mem_fetch(op)
                .map_err(|err| anyhow!("Expected constant.").context(err))?;

            let operation = match op_str.as_str() {
                "ADD" => PolyAsmInstruction::VectorAdd {
                    a: input!("A"),
                    b: input!("B"),
                    out_vec: output!("out_vec"),
                },
                "SUB" => PolyAsmInstruction::VectorSub {
                    a: input!("A"),
                    b: input!("B"),
                    out_vec: output!("out_vec"),
                },
                invalid => {
                    bail!("Invalid VectorMath operation: {}", invalid)
                }
            };

            program.add_operation(operation);
        }
        "MergeMeshes" => {
            let operation = PolyAsmInstruction::MergeMeshes {
                a: input!("A"),
                b: input!("B"),
                out_mesh: output!("out_mesh"),
            };
            program.add_operation(operation);
        }
        "ExportObj" => {
            let operation = PolyAsmInstruction::ExportObj {
                in_mesh: input!("mesh"),
                export_path: input!("export_path"),
            };
            program.add_operation(operation);
        }
        invalid => return Err(anyhow!("Unknown op_name {}", invalid)),
    }
    Ok(())
}

pub fn compile_graph(graph: &Graph, final_node: NodeId) -> Result<PolyAsmProgram> {
    let mut program = PolyAsmProgram::new();
    let mut outputs_cache = OutputsCache::default();

    gen_code_for_node(&mut program, &graph, final_node, &mut outputs_cache)?;
    Ok(program)
}
