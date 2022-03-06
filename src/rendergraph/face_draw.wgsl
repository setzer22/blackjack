#include <utils.wgsl>
#include <rend3_uniforms.wgsl>

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
};

struct FragmentOutput {
    [[location(0)]] color: vec4<f32>;
};

[[group(1), binding(0)]]
var<storage> faces: Vec3Array;
[[group(1), binding(1)]]
var<storage> normals: Vec3Array;
[[group(1), binding(2)]]
var<storage> colors: Vec3Array;

fn map(value: f32, min1: f32, max1: f32, min2: f32, max2: f32) -> f32 {
  return min2 + (value - min1) * (max2 - min2) / (max1 - min1);
}

[[stage(vertex)]]
fn vs_main(
    [[builtin(instance_index)]] instance_idx: u32,
    [[builtin(vertex_index)]] vertex_idx: u32,
) -> VertexOutput {
    let position = unpack_v3(faces.inner[instance_idx * 3u + vertex_idx]);
    let color = unpack_v3(colors.inner[instance_idx]);
    let normal = unpack_v3(normals.inner[instance_idx]);

    var output : VertexOutput;
    output.clip_position = uniforms.view_proj * vec4<f32>(position, 1.0);
    output.color = color;
    output.normal = normalize((uniforms.view_proj * vec4<f32>(normal, 1.0)).xyz);
    return output;
}

[[stage(fragment)]]
fn fs_main(input: VertexOutput) -> FragmentOutput {
    var out : FragmentOutput;
    //out.color = vec4<f32>(vec3<f32>(input.normal.z), 1.0);
    let shading = max(map(input.normal.z, 0.0, 1.0, 0.0, 1.0), 0.0);
    out.color = vec4<f32>(vec3<f32>(shading), 1.0);
    return out;
}

// WIP

// 1. Here's how to apply a matcap

// -- Vertex --
// vNormal = normalize(vec3(world * vec4(normal, 0.0)));

// -- Fragment --
// highp vec2 muv = vec2(view * vec4(normalize(vNormal), 0))*0.5+vec2(0.5,0.5);
// gl_FragColor = texture2D(matcapTexture, vec2(muv.x, 1.0-muv.y));

// 2. We then need to learn how to load textures, and bind them on the shader.

// Other TODOs:
// - Need some controls to toggle various different elements of the viewport display
// - Restore halfedge visualizations
// - Draw matcap images on egui (+++)
// - Face mesh indices -> Smooth normals
// - Face selection