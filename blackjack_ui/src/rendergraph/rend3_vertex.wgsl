struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) uv_0: vec2<f32>,
    @location(4) uv_1: vec2<f32>,
    @location(5) color: vec4<f32>,
};
