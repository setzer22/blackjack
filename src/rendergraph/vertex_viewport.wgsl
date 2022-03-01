#include <rend3_common.wgsl>

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0), interpolate(flat)]] color: vec4<f32>;
};

struct FragmentOutput {
    [[builtin(frag_depth)]] depth: f32;
    [[location(0)]] color: vec4<f32>;
};

struct VertexMaterial {
    base_color: vec4<f32>;
    thickness: f32;
};

[[group(2), binding(0)]]
var<storage> material: VertexMaterial;

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
    out.color = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    // We want edges slightly over their actual positions towards the camera.
    // This prevents z-fighting when drawing the wireframe over the mesh. The
    // value 1.02 is chosen to be slightly higher than the one for edges, to
    // ensure vertices are drawn over edges
    out.depth = input.clip_position.z * 1.02;
    return out;
}
