#include <rend3_common.wgsl>

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0), interpolate(flat)]] color: vec4<f32>;
};

struct FragmentOutput {
    [[builtin(frag_depth)]] depth: f32;
    [[location(0)]] color: vec4<f32>;
};

struct EdgeMaterial {
    base_color: vec4<f32>;
    thickness: f32;
    pad1: f32;
    pad2: f32;
    pad3: f32;
};

[[group(2), binding(0)]]
var<storage> material: EdgeMaterial;

[[stage(vertex)]]
fn vs_main(
    // In CPU Mode, the object index is (ab)using the instance index
    [[builtin(instance_index)]] object_idx: u32,
    vertex: VertexInput,
) -> VertexOutput {
    let object = object_data.objects[object_idx];
    var output : VertexOutput;
    output.clip_position = object.model_view_proj * vec4<f32>(vertex.position, 1.0);
    output.color = material.base_color;
    return output;
}

[[stage(fragment)]]
fn fs_main(input: VertexOutput) -> FragmentOutput {
    var out : FragmentOutput;
    out.color = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    // We want edges slightly over their actual positions towards the camera.
    // This prevents z-fighting when drawing the wireframe over the mesh.
    out.depth = input.clip_position.z * 1.01;
    return out;
}
