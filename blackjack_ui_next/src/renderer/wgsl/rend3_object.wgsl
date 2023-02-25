
struct ObjectData {
    model_view: mat4x4<f32>,
    model_view_proj: mat4x4<f32>,
    // NOTE: This is unused in GPU mode
    material_idx: u32,
    inv_squared_scale: vec3<f32>,
};


struct ObjectDataArray { 
    objects: array<ObjectData>,
};


@group(1) @binding(0)
var<storage> object_data: ObjectDataArray;
