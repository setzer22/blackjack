use crate::prelude::*;
use slotmap::SlotMap;

use crate::prelude::selection::SelectionExpression;

slotmap::new_key_type! { pub struct BjkNodeId; }

pub enum ConstantValue {
    Vector(glam::Vec3),
    Scalar(f32),
    String(String),
    Selection(SelectionExpression),
}

/// The data types that can exist inside a Blackjack node graph
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DataType {
    Vector,
    Scalar,
    Selection,
    Mesh,
    String,
}

pub struct LuaExpression(String);

pub enum DependencyKind {
    Computed(LuaExpression),
    External,
    Connection { node: BjkNodeId, param_name: String },
}
pub struct Input {
    pub name: String,
    pub data_type: DataType,
    pub kind: DependencyKind,
}

pub struct Output {
    pub name: String,
    pub data_type: DataType,
}

pub struct BjkNode {
    pub op_name: String,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}

#[derive(Default)]
pub struct BjkGraph {
    pub nodes: SlotMap<BjkNodeId, BjkNode>,
}

impl BjkGraph {
    // Constructs an empty graph
    pub fn new() -> Self {
        Self {
            nodes: Default::default(),
        }
    }
    /// Adds a new empty node to the graph
    pub fn add_node(&mut self, op_name: impl ToString) -> BjkNodeId {
        self.nodes.insert(BjkNode {
            op_name: op_name.to_string(),
            inputs: vec![],
            outputs: vec![],
        })
    }

    /// Registers a new input for `node_id`
    pub fn add_input(
        &mut self,
        node_id: BjkNodeId,
        name: impl ToString,
        data_type: DataType,
    ) -> Result<()> {
        let name = name.to_string();
        let node = &mut self.nodes[node_id];
        if node.inputs.iter().any(|input| input.name == name) {
            bail!("Input parameter {name} already exists for node {node_id:?}");
        } else {
            self.nodes[node_id].inputs.push(Input {
                name,
                data_type,
                kind: DependencyKind::External,
            });
        }
        Ok(())
    }

    pub fn add_output(
        &mut self,
        node_id: BjkNodeId,
        name: impl ToString,
        data_type: DataType,
    ) -> Result<()> {
        let name = name.to_string();
        let node = &mut self.nodes[node_id];
        if node.outputs.iter().any(|output| output.name == name) {
            bail!("Output parameter {name} already exists for node {node_id:?}");
        } else {
            self.nodes[node_id].outputs.push(Output { name, data_type });
        }
        Ok(())
    }

    pub fn add_connection(
        &mut self,
        src_node: BjkNodeId,
        src_param: &str,
        dst_node: BjkNodeId,
        dst_param: &str,
    ) -> Result<()> {
        let src_data_type = self.nodes[src_node]
            .outputs
            .iter()
            .find(|output| output.name == src_param)
            .map(|output| output.data_type)
            .ok_or_else(|| {
                anyhow!("Input parameter named {dst_param} does not exist for node {dst_node:?}")
            })?;

        if let Some(input) = self.nodes[dst_node]
            .inputs
            .iter_mut()
            .find(|input| input.name == dst_param)
        {
            if input.data_type != src_data_type {
                bail!(
                    "Incompatible types. Input is {:?}, but its corresponding output is {:?}",
                    input.data_type,
                    src_data_type
                );
            }

            input.kind = DependencyKind::Connection {
                node: src_node,
                param_name: src_param.into(),
            }
        } else {
            bail!("Input parameter named {dst_param} does not exist for node {dst_node:?}");
        }
        Ok(())
    }
}

pub mod test_compiler;
