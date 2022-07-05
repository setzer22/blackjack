#include <utils.wgsl>
#include <rend3_uniforms.wgsl>

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec3<f32>;
};

struct FragmentOutput {
    [[location(0)]] color: vec4<f32>;
};

[[group(1), binding(0)]]
var<storage> positions: Vec3Array;
[[group(1), binding(1)]]
var<storage> colors: Vec3Array;

[[stage(vertex)]]
fn vs_main(
    [[builtin(instance_index)]] instance_idx: u32,
    [[builtin(vertex_index)]] vertex_idx: u32,
) -> VertexOutput {
    let position = unpack_v3(positions.inner[instance_idx * 3u + vertex_idx]);
    let color = unpack_v3(colors.inner[instance_idx]);

    var output : VertexOutput;
    output.clip_position = uniforms.view_proj * vec4<f32>(position, 1.0);
    output.color = color;
    return output;
}

[[stage(fragment)]]
fn fs_main(input: VertexOutput) -> FragmentOutput {
    var out : FragmentOutput;
    out.color = vec4<f32>(input.color, 0.5);
    return out;
}