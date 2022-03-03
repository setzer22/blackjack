#include <rend3_uniforms.wgsl>

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
};

struct FragmentOutput {
    [[builtin(frag_depth)]] depth: f32;
    [[location(0)]] color: vec4<f32>;
};

// We want the right stride for vec3.
struct PackedVec3 {
    x: f32; y: f32; z: f32;
};

struct PointCloudArray {
    inner: array<PackedVec3>;
};

[[group(1), binding(0)]]
var<storage> point_cloud: PointCloudArray;

var<private> quad: array<vec3<f32>,6> = array<vec3<f32>,6>(
    vec3<f32>(-1.0, 1.0, 0.0), // 0
    vec3<f32>(1.0, -1.0, 0.0), // 3
    vec3<f32>(1.0, 1.0, 0.0), // 1
    vec3<f32>(-1.0, 1.0, 0.0), // 0
    vec3<f32>(-1.0, -1.0, 0.0), // 2
    vec3<f32>(1.0, -1.0, 0.0), // 3
);

[[stage(vertex)]]
fn vs_main(
    [[builtin(instance_index)]] instance_idx: u32,
    [[builtin(vertex_index)]] vertex_idx: u32,
) -> VertexOutput {
    let point = point_cloud.inner[instance_idx];
    let point = vec3<f32>(point.x, point.y, point.z);
    let quad_vertex = quad[vertex_idx];
    let position = point + quad_vertex * 0.1;

    var output : VertexOutput;
    output.clip_position = uniforms.view_proj * vec4<f32>(position, 1.0);
    return output;
}

[[stage(fragment)]]
fn fs_main(input: VertexOutput) -> FragmentOutput {
    var out : FragmentOutput;
    out.color = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    // We want edges slightly over their actual positions towards the camera.
    // This prevents z-fighting when drawing the wireframe over the mesh.
    out.depth = input.clip_position.z * 1.02;
    return out;
}
