#include <utils.wgsl>
#include <uniforms.wgsl>
#include <gizmo_common.wgsl>

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) id: u32,
};

struct FragmentOutput {
    @location(0) id: u32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
) -> VertexOutput {
    let position = unpack_v3(positions.inner[vertex_idx]);
    let subgizmo_id = subgizmo_ids[vertex_idx];
    let subgizmo = subgizmos[subgizmo_id];
   
    var output : VertexOutput;
    
    // Make the gizmo scale independent from camera position
    let scale_adj = adjust_scale(
        uniforms.inv_view,
        vec3(0.0, 0.0, 0.0), // TODO: Use gizmo position!!
        f32(uniforms.resolution.y),
        50.0 // Gizmo size
    );
    
    output.clip_position = uniforms.view_proj * scale_adj * vec4<f32>(position, 1.0);

    output.id = subgizmo.object_pick_id;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    var out : FragmentOutput;
    out.id = input.id;
    return out;
}
