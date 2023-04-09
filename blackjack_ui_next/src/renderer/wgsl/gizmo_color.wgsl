#include <utils.wgsl>
#include <uniforms.wgsl>
#include <gizmo_common.wgsl>

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) color: vec3<f32>,
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
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

    if subgizmo.is_highlighted == 0u {
        output.color = unpack_v3(subgizmo.color);
    } else {
        output.color = vec3(1.0, 1.0, 1.0);
    }
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    var out : FragmentOutput;
    out.color = vec4(input.color, 0.0);
    return out;
}
