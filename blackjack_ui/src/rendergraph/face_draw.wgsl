#include <utils.wgsl>
#include <rend3_uniforms.wgsl>

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) normal: vec3<f32>,
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

@group(1) @binding(0)
var<storage> positions: Vec3Array;
@group(1) @binding(1)
var<storage> normals: Vec3Array;
@group(1) @binding(2)
var matcap: texture_2d<f32>;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
) -> VertexOutput {
    let position = unpack_v3(positions.inner[vertex_idx]);
    let normal = unpack_v3(normals.inner[vertex_idx]);

    var output : VertexOutput;
    output.clip_position = uniforms.view_proj * vec4<f32>(position, 1.0);
    output.normal = normalize(normal);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    var out : FragmentOutput;

    let muv = (uniforms.view * vec4<f32>(normalize(input.normal), 0.0)).xy;
    let muv = muv * 0.5 + vec2<f32>(0.5, 0.5);

    out.color = textureSample(matcap, primary_sampler, vec2<f32>(muv.x, 1.0 - muv.y));

    return out;
}
