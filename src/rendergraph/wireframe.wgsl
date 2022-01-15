
struct ObjectOutputData {
    model_view: mat4x4<f32>;
    model_view_proj: mat4x4<f32>;
    material_idx: u32;
    inv_squared_scale: vec3<f32>;
};

[[block]]
struct ObjectOutputDataBuffer {
    object_output: [[stride(160)]] array<ObjectOutputData>;
};

[[block]]
struct gl_PerVertex {
    [[builtin(position)]] gl_Position: vec4<f32>;
};

struct VertexOutput {
    [[location(0)]] member: vec4<f32>;
    [[builtin(position)]] gl_Position: vec4<f32>;
    [[location(3)]] member_1: u32;
    [[location(2)]] member_2: vec4<f32>;
    [[location(1)]] member_3: vec2<f32>;
};

var<private> gl_InstanceIndex_1: i32;
[[group(0), binding(0)]]
var<storage> unnamed: ObjectOutputDataBuffer;
var<private> i_position_1: vec3<f32>;
var<private> o_position: vec4<f32>;
var<private> perVertexStruct: gl_PerVertex = gl_PerVertex(vec4<f32>(0.0, 0.0, 0.0, 1.0), );
var<private> o_material: u32;
var<private> o_color: vec4<f32>;
var<private> i_color_1: vec4<f32>;
var<private> o_coords0_: vec2<f32>;
var<private> i_coords0_1: vec2<f32>;
var<private> i_normal_1: vec3<f32>;
var<private> i_tangent_1: vec3<f32>;
var<private> i_coords1_1: vec2<f32>;
var<private> i_material_1: u32;

fn main_1() {
    let e22: i32 = gl_InstanceIndex_1;
    let e27: mat4x4<f32> = unnamed.object_output[bitcast<u32>(e22)].model_view_proj;
    let e29: u32 = unnamed.object_output[bitcast<u32>(e22)].material_idx;
    let e30: vec3<f32> = i_position_1;
    let e35: vec4<f32> = (e27 * vec4<f32>(e30.x, e30.y, e30.z, 1.0));
    o_position = e35;
    perVertexStruct.gl_Position = e35;
    o_material = e29;
    let e37: vec4<f32> = i_color_1;
    o_color = e37;
    let e38: vec2<f32> = i_coords0_1;
    o_coords0_ = e38;
    return;
}

[[stage(vertex)]]
fn vertex_main([[builtin(instance_index)]] gl_InstanceIndex: u32, [[location(0)]] i_position: vec3<f32>, [[location(5)]] i_color: vec4<f32>, [[location(3)]] i_coords0_: vec2<f32>, [[location(1)]] i_normal: vec3<f32>, [[location(2)]] i_tangent: vec3<f32>, [[location(4)]] i_coords1_: vec2<f32>, [[location(6)]] i_material: u32) -> VertexOutput {
    gl_InstanceIndex_1 = i32(gl_InstanceIndex);
    i_position_1 = i_position;
    i_color_1 = i_color;
    i_coords0_1 = i_coords0_;
    i_normal_1 = i_normal;
    i_tangent_1 = i_tangent;
    i_coords1_1 = i_coords1_;
    i_material_1 = i_material;
    main_1();
    let e23: vec4<f32> = o_position;
    let e24: vec4<f32> = perVertexStruct.gl_Position;
    let e25: u32 = o_material;
    let e26: vec4<f32> = o_color;
    let e27: vec2<f32> = o_coords0_;
    return VertexOutput(e23, e24, e25, e26, e27);
}


[[stage(fragment)]]
fn fragment_main([[location(3)]] i_coords0_: vec2<f32>, [[location(5)]] i_color: vec4<f32>, [[location(1)]] i_normal: vec3<f32>, [[location(2)]] i_tangent: vec3<f32>, [[location(0)]] i_view_position: vec4<f32>, [[location(4)]] i_coords1_: vec2<f32>, [[location(6)]] i_material: u32) -> [[location(0)]] vec4<f32> {
    return vec4(1.0, 1.0, 1.0, 1.0);
}
