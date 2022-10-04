#include <utils.wgsl>
#include <rend3_uniforms.wgsl>

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct FragmentOutput {
    @builtin(frag_depth) depth: f32,
    @location(0) color: vec4<f32>,
};

@group(1) @binding(0)
var<storage> lines: Vec3Array;

@group(1) @binding(1)
var<storage> colors: Vec3Array;

@vertex
fn vs_main(
    @builtin(instance_index) instance_idx: u32,
    @builtin(vertex_index) vertex_idx: u32,
) -> VertexOutput {
    var current_point = unpack_v3(lines.inner[instance_idx * 2u + vertex_idx]);
    var color = unpack_v3(colors.inner[instance_idx]);

    var output : VertexOutput;
    output.clip_position = uniforms.view_proj * vec4<f32>(current_point, 1.0);
    output.color = color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    var out : FragmentOutput;
    out.color = vec4<f32>(input.color, 1.0);
    // We want edges slightly over their actual positions towards the camera.
    // This prevents z-fighting when drawing the wireframe over the mesh.
    out.depth = input.clip_position.z * 1.01;
    return out;
}
