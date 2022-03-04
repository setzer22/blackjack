#include <utils.wgsl>
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

var<private> screen_quad: array<vec2<f32>, 6> = array<vec2<f32>, 6>( 
    vec2<f32>(0.0, 1.0),
    vec2<f32>(-1.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(-1.0, 0.0),
    vec2<f32>(0.0, -1.0),
    vec2<f32>(1.0, 0.0),
);

[[stage(vertex)]]
fn vs_main(
    [[builtin(instance_index)]] instance_idx: u32,
    [[builtin(vertex_index)]] vertex_idx: u32,
) -> VertexOutput {
    // Get the current point
    let point = unpack_v3(point_cloud.inner[instance_idx]);

    // Compute its clip space position
    let point_clip = uniforms.view_proj * vec4<f32>(point, 1.0);

    // Get the offset for the current vertex in the quad
    let screen_quad_vertex = screen_quad[vertex_idx];
    let pixel_size = vec2<f32>(1.0 / f32(uniforms.resolution.x), 1.0 / f32(uniforms.resolution.y));
    let point_size = pixel_size * 8.0;
    let vertex_offset = screen_quad_vertex * point_size;

    // The final position is the clip space position for the point, plus the
    // screen-space quad offset.
    let clip_position = (point_clip / point_clip.w) + vec4<f32>(vertex_offset, 0.0, 0.0);; 

    var output : VertexOutput;
    output.clip_position = clip_position;
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
