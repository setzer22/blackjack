// Vertex shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) near_point: vec3<f32>,
    @location(1) far_point: vec3<f32>,
};

struct FragmentOutput {
    @builtin(frag_depth) depth: f32,
    @location(0) color: vec4<f32>,
};

struct GridRoutineUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> matrices: GridRoutineUniform;

var<private> vertices: array<vec2<f32>, 6> = array<vec2<f32>, 6>( 
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(-1.0, -1.0),
);

fn unproject_point(projected_point: vec3<f32>, inv_view: mat4x4<f32>, inv_proj: mat4x4<f32>) -> vec3<f32> {
    let unprojected_point =  inv_view * inv_proj * vec4<f32>(projected_point, 1.0);
    return unprojected_point.xyz / unprojected_point.w;
}

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let pos_xy = vertices[in_vertex_index];
    let pos = vec4<f32>(pos_xy.x, pos_xy.y, 0.0, 1.0);

    out.clip_position = pos;
    // TODO: Compute near_point / far_point
    out.near_point = unproject_point(vec3<f32>(pos.x, pos.y, 0.1), matrices.inv_view, matrices.inv_proj).xyz;
    out.far_point = unproject_point(vec3<f32>(pos.x, pos.y, 1.0), matrices.inv_view, matrices.inv_proj).xyz;

    return out;
}

// Fragment shader

fn grid(frag_pos_3d: vec3<f32>, scale: f32) -> vec4<f32> {
    let coord = frag_pos_3d.xz * scale; // use the scale variable to set the distance between the lines
    let derivative = fwidth(coord);
    let grid = abs(fract(coord - 0.5) - 0.5) / derivative;
    let grid_line = min(grid.x, grid.y);
    let minimumz = min(derivative.y, 1.0);
    let minimumx = min(derivative.x, 1.0);
    var color = vec4<f32>(0.2, 0.2, 0.2, 1.0 - min(grid_line, 1.0));

    let threshold = 1.0 / scale;

    // z axis
    if (frag_pos_3d.x > -threshold * minimumx && frag_pos_3d.x < threshold * minimumx) {
        color.z = 1.0;
    }
    // x axis
    if (frag_pos_3d.z > -threshold * minimumz && frag_pos_3d.z < threshold * minimumz) {
        color.x = 1.0;
    }
    return color;
}

fn compute_depth(frag_pos_3d: vec3<f32>) -> f32 {
    let clip_space_pos = matrices.proj * matrices.view * vec4<f32>(frag_pos_3d, 1.0);
    return (clip_space_pos.z / clip_space_pos.w);
}

fn fading(frag_pos_3d: vec3<f32>, depth: f32) -> f32 {
    let znear = 0.001;
    // If you're using far plane at infinity as described here, then linearized depth is simply znear / depth.
    // From: https://www.reddit.com/r/GraphicsProgramming/comments/f9zwin/linearising_reverse_depth_buffer/
    let linear_depth = znear / depth;
    return max(0.0, 2.5 - linear_depth);
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let t = -in.near_point.y / (in.far_point.y - in.near_point.y);
    let frag_pos_3d = in.near_point + t * (in.far_point - in.near_point);

    let depth = compute_depth(frag_pos_3d);

    var out: FragmentOutput;
    out.color = grid(frag_pos_3d, 2.0) * f32(t < 0.0);
    out.depth = depth;
    out.color.a = out.color.a * fading(frag_pos_3d, depth);

    return out;
}
