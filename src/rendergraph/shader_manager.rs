use std::collections::HashMap;

use wgpu::{ColorTargetState, FragmentState, VertexBufferLayout, VertexState};

pub struct Shader {
    pub fs_entry_point: String,
    pub vs_entry_point: String,
    pub module: wgpu::ShaderModule,
}

impl Shader {
    pub fn to_vertex_state<'a>(&'a self, buffers: &'a [VertexBufferLayout]) -> VertexState {
        VertexState {
            module: &self.module,
            entry_point: &self.vs_entry_point,
            buffers,
        }
    }

    pub fn to_fragment_state<'a>(&'a self, targets: &'a [ColorTargetState]) -> FragmentState {
        FragmentState {
            module: &self.module,
            entry_point: &self.fs_entry_point,
            targets,
        }
    }
}

pub struct ShaderManager {
    pub shaders: HashMap<String, Shader>,
}

impl ShaderManager {
    pub fn new(device: &wgpu::Device) -> Self {
        let mut shaders = HashMap::new();

        let mut context = glsl_include::Context::new();
        let context = context
            .include("rend3_common.wgsl", include_str!("rend3_common.wgsl"))
            .include("rend3_vertex.wgsl", include_str!("rend3_vertex.wgsl"))
            .include("rend3_object.wgsl", include_str!("rend3_object.wgsl"))
            .include("rend3_uniforms.wgsl", include_str!("rend3_uniforms.wgsl"));

        macro_rules! def_shader {
            ($name:expr, $src:expr) => {
                shaders.insert(
                    $name.to_string(),
                    Shader {
                        fs_entry_point: "fs_main".into(),
                        vs_entry_point: "vs_main".into(),
                        module: device.create_shader_module(&wgpu::ShaderModuleDescriptor {
                            label: Some($name),
                            source: wgpu::ShaderSource::Wgsl(
                                context
                                    .expand(include_str!($src))
                                    .expect("Shader preprocessor")
                                    .into(),
                            ),
                        }),
                    },
                );
            };
        }

        def_shader!("edge_viewport", "edge_viewport.wgsl");
        def_shader!("point_cloud_draw", "point_cloud_draw.wgsl");

        Self { shaders }
    }

    pub fn get(&self, shader_name: &str) -> &Shader {
        self.shaders.get(shader_name).unwrap()
    }
}
