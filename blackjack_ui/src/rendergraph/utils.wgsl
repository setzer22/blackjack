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

struct U32Array {
    inner: array<u32>,
};

struct ColorArray {
    // Unlike vec3, vec4 has the same stride and alignment of 16, so we don't
    // need a Packed version
    inner: array<vec4<f32>>
};

// Simple RNG. Borrowed from: https://stackoverflow.com/a/17479300/
// Translated to WGSL. Credit goes to original authors.

fn hash(x: u32) -> u32 {
    var x = x;
    x += ( x << 10u );
    x ^= ( x << 6u );
    x += ( x << 3u );
    x ^= ( x << 11u );
    x += ( x << 15u );
    return x;
}

fn hash_v2(v: vec2<u32>) -> u32 {
    return hash(v.x ^ hash(v.y));
}

fn hash_v3(v: vec3<u32>) -> u32 {
    return hash(v.x ^ hash(v.y) ^ hash(v.z));
}

fn hash_v4(v: vec4<u32>) -> u32 {
    return hash(v.x ^ hash(v.y) ^ hash(v.z) ^ hash(v.w));
}

fn float_construct(m: u32) -> f32 {
    var m = m;
    let ieee_mantissa = 0x007FFFFFu;
    let ieee_one = 0x3F800000u;
     m &= ieee_mantissa;
     m |= ieee_one;

     let f = bitcast<f32>(m);
     return f - 1.0;
}

fn random(x: f32) -> f32 {
    return float_construct(hash(bitcast<u32>(x)));
}

fn random_v2(v: vec2<f32>) -> f32 {
    return float_construct(hash_v2(bitcast<vec2<u32>>(v)));
}

fn random_v3(v: vec3<f32>) -> f32 {
    return float_construct(hash_v3(bitcast<vec3<u32>>(v)));
}

fn random_v4(v: vec4<f32>) -> f32 {
    return float_construct(hash_v4(bitcast<vec4<u32>>(v)));
}
