// Sometimes we have a packed array of Vec3 and we want them to have an
// alignment of 12, but vec3 aligns at 16. This struct gets around that
// limitation. Use the `from_packed_v3` and `to_packed_v3` to convert.
struct PackedVec3 {
    x: f32,
    y: f32,
    z: f32,
};

/// Packs a vec3 into a PackedVec3
fn pack_v3(v3: vec3<f32>) -> PackedVec3 {
    var out: PackedVec3;
    out.x = v3.x;
    out.y = v3.y;
    out.z = v3.z;
    return out;
}

/// Unpacks a PackedVec3 into a vec3
fn unpack_v3(v3: PackedVec3) -> vec3<f32> {
    return vec3<f32>(v3.x, v3.y, v3.z);
}

/// WGSL does not allow declaring storage arrays directly, so we need a wrapper
/// struct to hold them.
struct Vec3Array {
    inner: array<PackedVec3>,
};