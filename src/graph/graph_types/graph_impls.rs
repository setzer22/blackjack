use smallvec::smallvec;

use super::*;
use crate::prelude::*;

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, d: NodeDescriptor) -> NodeId {
        let node_id = self.nodes.insert_with_key(|node_id| {
            Node {
                id: node_id,
                label: d.label,
                op_name: d.op_name,
                // These will get filled in later
                inputs: Vec::default(),
                outputs: Vec::default(),
                is_executable: d.is_executable,
            }
        });

        use super::InputParamKind::*;
        let inputs: Vec<(String, InputId)> = d
            .inputs
            .into_iter()
            .map(|(input_name, input)| {
                let input_id = self.inputs.insert_with_key(|id| match input {
                    InputDescriptor::Vector { default } => InputParam {
                        id,
                        typ: DataType::Vector,
                        value: InputParamValue::Vector(default),
                        metadata: smallvec![],
                        kind: ConnectionOrConstant,
                        node: node_id,
                    },
                    InputDescriptor::Mesh => InputParam {
                        id,
                        typ: DataType::Mesh,
                        value: InputParamValue::None,
                        metadata: smallvec![],
                        kind: ConnectionOnly,
                        node: node_id,
                    },
                    InputDescriptor::Selection => InputParam {
                        id,
                        typ: DataType::Selection,
                        value: InputParamValue::Selection {
                            text: "".into(),
                            selection: Some(vec![]),
                        },
                        metadata: smallvec![],
                        kind: ConnectionOrConstant,
                        node: node_id,
                    },
                    InputDescriptor::Scalar { default, min, max } => InputParam {
                        id,
                        typ: DataType::Scalar,
                        value: InputParamValue::Scalar(default),
                        metadata: smallvec![InputParamMetadata::MinMaxScalar { min, max }],
                        kind: ConnectionOrConstant,
                        node: node_id,
                    },
                    InputDescriptor::Enum { values } => InputParam {
                        id,
                        typ: DataType::Enum,
                        value: InputParamValue::Enum {
                            values,
                            selection: None,
                        },
                        metadata: smallvec![],
                        kind: ConstantOnly,
                        node: node_id,
                    },
                    InputDescriptor::NewFile => InputParam {
                        id,
                        typ: DataType::NewFile,
                        value: InputParamValue::NewFile { path: None },
                        metadata: smallvec![],
                        kind: ConstantOnly,
                        node: node_id,
                    },
                });
                (input_name, input_id)
            })
            .collect();

        let outputs: Vec<(String, OutputId)> = d
            .outputs
            .into_iter()
            .map(|(output_name, output)| {
                let output_id = self.outputs.insert_with_key(|id| OutputParam {
                    node: node_id,
                    id,
                    typ: output.0,
                });
                (output_name, output_id)
            })
            .collect();

        self[node_id].inputs = inputs;
        self[node_id].outputs = outputs;
        node_id
    }

    pub fn remove_node(&mut self, node_id: NodeId) {
        self.connections
            .retain(|i, o| !(self.outputs[*o].node == node_id || self.inputs[*i].node == node_id));
        let inputs: SVec<_> = self[node_id].input_ids().collect();
        for input in inputs {
            self.inputs.remove(input);
        }
        let outputs: SVec<_> = self[node_id].output_ids().collect();
        for output in outputs {
            self.outputs.remove(output);
        }
        self.nodes.remove(node_id);
    }

    pub fn remove_connection(&mut self, input_id: InputId) -> Option<OutputId> {
        self.connections.remove(&input_id)
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.iter().map(|(id, _)| id)
    }

    pub fn add_connection(&mut self, output: OutputId, input: InputId) {
        self.connections.insert(input, output);
    }

    pub fn iter_connections(&self) -> impl Iterator<Item = (InputId, OutputId)> + '_ {
        self.connections.iter().map(|(o, i)| (*o, *i))
    }

    pub fn connection(&self, input: InputId) -> Option<OutputId> {
        self.connections.get(&input).copied()
    }

    pub fn any_param_type(&self, param: AnyParameterId) -> Result<DataType> {
        match param {
            AnyParameterId::Input(input) => self.inputs.get(input).map(|x| x.typ),
            AnyParameterId::Output(output) => self.outputs.get(output).map(|x| x.typ),
        }
        .ok_or_else(|| anyhow!("Invalid parameter id: {:?}", param))
    }

    pub fn get_input(&self, input: InputId) -> &InputParam {
        &self.inputs[input]
    }

    pub fn get_output(&self, output: OutputId) -> &OutputParam {
        &self.outputs[output]
    }
}

impl Node {
    pub fn inputs<'a>(&'a self, graph: &'a Graph) -> impl Iterator<Item = &InputParam> + 'a {
        self.input_ids().map(|id| graph.get_input(id))
    }

    pub fn outputs<'a>(&'a self, graph: &'a Graph) -> impl Iterator<Item = &OutputParam> + 'a {
        self.output_ids().map(|id| graph.get_output(id))
    }

    pub fn input_ids(&self) -> impl Iterator<Item = InputId> + '_ {
        self.inputs.iter().map(|(_name, id)| *id)
    }

    pub fn output_ids(&self) -> impl Iterator<Item = OutputId> + '_ {
        self.outputs.iter().map(|(_name, id)| *id)
    }

    pub fn get_input(&self, name: &str) -> Result<InputId> {
        self.inputs
            .iter()
            .find(|(param_name, _id)| param_name == name)
            .map(|x| x.1)
            .ok_or_else(|| anyhow!("Node {:?} has no parameter named {}", self.id, name))
    }

    pub fn get_output(&self, name: &str) -> Result<OutputId> {
        self.outputs
            .iter()
            .find(|(param_name, _id)| param_name == name)
            .map(|x| x.1)
            .ok_or_else(|| anyhow!("Node {:?} has no parameter named {}", self.id, name))
    }

    /// Can this node be enabled on the UI? I.e. does it output a mesh?
    pub fn can_be_enabled(&self, graph: &Graph) -> bool {
        self.outputs(graph)
            .any(|output| output.typ == DataType::Mesh)
    }

    /// Executable nodes are used to produce side effects, like exporting files.
    pub fn is_executable(&self) -> bool {
        self.is_executable
    }
}
