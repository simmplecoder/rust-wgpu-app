@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var destination_texture: texture_storage_2d<rgba8unorm, write>;

fn luma709(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn sample_luma_clamped(coord: vec2<i32>, dims: vec2<u32>) -> f32 {
    let min_xy = vec2<i32>(0, 0);
    let max_xy = vec2<i32>(i32(dims.x) - 1, i32(dims.y) - 1);
    let clamped = clamp(coord, min_xy, max_xy);
    let px = textureLoad(source_texture, clamped, 0);
    return luma709(px.rgb);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(destination_texture);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let c = vec2<i32>(gid.xy);

    // 3x3 neighborhood luma samples around current pixel.
    let s00 = sample_luma_clamped(c + vec2<i32>(-1, -1), dims);
    let s01 = sample_luma_clamped(c + vec2<i32>( 0, -1), dims);
    let s02 = sample_luma_clamped(c + vec2<i32>( 1, -1), dims);
    let s10 = sample_luma_clamped(c + vec2<i32>(-1,  0), dims);
    let s11 = sample_luma_clamped(c + vec2<i32>( 0,  0), dims);
    let s12 = sample_luma_clamped(c + vec2<i32>( 1,  0), dims);
    let s20 = sample_luma_clamped(c + vec2<i32>(-1,  1), dims);
    let s21 = sample_luma_clamped(c + vec2<i32>( 0,  1), dims);
    let s22 = sample_luma_clamped(c + vec2<i32>( 1,  1), dims);

    let gx = s00 * -1.0f + s10 * -2.0f + s20 * -1.0f + s02 * 1.0f + s12 * 2.0f + s22 * 1.0f;
    let gy = s00 * -1.0f + s01 * -2.0f + s02 * -1.0f + s20 * 1.0f + s21 * 2.0f + s22 * 1.0f;

    // TODO: tune normalization; 4.0 is a starting guess for visibility.
    let edge = clamp(length(vec2<f32>(gx, gy)) / 4.0, 0.0, 1.0);

    // Temporary fallback display so shader compiles while you iterate:
    // replace `edge` with `s11` if you want grayscale preview.
    textureStore(destination_texture, c, vec4<f32>(edge, edge, edge, 1.0));
}
