struct SubGizmo {
    color: PackedVec3,
    object_pick_id: u32,
    is_highlighted: u32,    
}

@group(1) @binding(0)
var<storage> positions: Vec3Array;
@group(1) @binding(1)
var<storage> subgizmo_ids: array<u32>;
@group(1) @binding(2)
var<storage> subgizmos: array<SubGizmo>;

// Computes a scale adjustment matrix so that the gizmo size remains constant
// independently of the camera distance
fn adjust_scale(
    inv_view: mat4x4<f32>, 
    gizmo_position: vec3<f32>, 
    screen_height: f32,
    gizmo_size: f32,
) -> mat4x4<f32> {
    let camera_position: vec3<f32> = inv_view[3].xyz;
    let distance: f32 = distance(gizmo_position, camera_position);
    let scale_factor: f32 = gizmo_size * distance / screen_height;
    return mat4x4(
        vec4(scale_factor, 0.0, 0.0, 0.0),
        vec4(0.0, scale_factor, 0.0, 0.0),
        vec4(0.0, 0.0, scale_factor, 0.0),
        vec4(0.0, 0.0, 0.0, 1.0),
    );
}