struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] tangent: vec3<f32>;
    [[location(3)]] uv_0: vec2<f32>;
    [[location(4)]] uv_1: vec2<f32>;
    [[location(5)]] color: vec4<f32>;
};

struct Plane {
    inner: vec4<f32>;
};

struct Frustrum {
    left: Plane;
    right: Plane;
    top: Plane;
    bottom: Plane;
    // No far plane
    near: Plane;
};

struct UniformData {
    view: mat4x4<f32>;
    view_proj: mat4x4<f32>;
    origin_view_proj: mat4x4<f32>;
    inv_view: mat4x4<f32>;
    inv_view_proj: mat4x4<f32>;
    inv_origin_view_proj: mat4x4<f32>;
    frustrum: Frustrum;
    ambient: vec4<f32>;
};

struct ObjectData {
    model_view: mat4x4<f32>;
    model_view_proj: mat4x4<f32>;
    // NOTE: This is unused in GPU mode
    material_idx: u32;
    inv_squared_scale: vec3<f32>;
};

struct ObjectDataArray { 
    objects: array<ObjectData>;
};

[[group(0), binding(0)]]
var primary_sampler: sampler;

[[group(0), binding(3)]]
var<uniform> uniforms: UniformData;


[[group(1), binding(0)]]
var<storage> object_data: ObjectDataArray;