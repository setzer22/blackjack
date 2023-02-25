/// NOTE: Must match definitions in render_state.rs
struct ViewportUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    resolution: vec2<u32>,
    padding0: vec2<u32>,
    padding1: vec2<u32>,
    padding2: vec2<u32>,
};

@group(0) @binding(0)
var primary_sampler: sampler;

@group(0) @binding(1)
var<uniform> uniforms: ViewportUniforms;
