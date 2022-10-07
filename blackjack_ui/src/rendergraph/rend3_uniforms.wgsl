struct Plane {
    inner: vec4<f32>,
};

struct Frustrum {
    left: Plane,
    right: Plane,
    top: Plane,
    bottom: Plane,
    // No far plane
    near: Plane,
};

struct UniformData {
    view: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    origin_view_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    inv_origin_view_proj: mat4x4<f32>,
    frustrum: Frustrum,
    ambient: vec4<f32>,
    resolution: vec2<u32>,
};

@group(0) @binding(0)
var primary_sampler: sampler;

@group(0) @binding(3)
var<uniform> uniforms: UniformData;
