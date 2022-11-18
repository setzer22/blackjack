#include <utils.wgsl>
#include <rend3_uniforms.wgsl>

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) @interpolate(flat) id: u32,
};

struct FragmentOutput {
    // First output buffer is the id map
    @location(0) id: u32,
    // Second output buffer is a color map, only used for debugging
    //
    // TODO: Once this is verified to work well, we can probably remove it to
    // improve performance.
    @location(1) debug_color: vec4<f32>,
};

@group(1) @binding(0)
var<storage> positions: Vec3Array;
@group(1) @binding(1)
var<storage> ids: U32Array;
@group(2) @binding(0)
var<uniform> max_id: u32;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
) -> VertexOutput {
    let position = unpack_v3(positions.inner[vertex_idx]);

    var output : VertexOutput;
    output.clip_position = uniforms.view_proj * vec4<f32>(position, 1.0);
    output.id = ids.inner[vertex_id]
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    var out : FragmentOutput;

    out.id = input.id;

    let grayscale = input.id as f32 / max_id as f32;
    out.debug_color = vec4<f32>(grayscale, grayscale, grayscale, 1.0)

    return out;
}
