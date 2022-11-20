#include <utils.wgsl>
#include <rend3_uniforms.wgsl>

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) @interpolate(flat) id: u32,
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) id: u32,
};

@group(1) @binding(0)
var<storage> positions: Vec3Array;
@group(1) @binding(1)
var<storage> colors: ColorArray;
@group(1) @binding(2)
var<storage> ids: U32Array;
@group(1) @binding(3)
var<uniform> max_id: u32;

@vertex
fn vs_main(
    @builtin(instance_index) instance_idx: u32,
    @builtin(vertex_index) vertex_idx: u32,
) -> VertexOutput {
    let position = unpack_v3(positions.inner[instance_idx * 3u + vertex_idx]);
    let color = colors.inner[instance_idx];
    let id = ids.inner[instance_idx];

    var output : VertexOutput;
    output.clip_position = uniforms.view_proj * vec4<f32>(position, 1.0);
    output.color = color;
    output.id = id;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    var out : FragmentOutput;
    out.color = input.color;
    out.id = input.id;

    // Debug: Use random colors for each id
    /*let t = f32(input.id + 1u) / f32(max_id);
    let color = vec3<f32>(
        random(t + 2.1923810),
        random(t + 4.2123190),
        random(t + 3.5132098),
    );
    out.color = vec4<f32>(color, 1.0);
    */

    return out;
}
