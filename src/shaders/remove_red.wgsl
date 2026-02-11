@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var destination_texture: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dimensions = textureDimensions(destination_texture);
    if (gid.x >= dimensions.x || gid.y >= dimensions.y) {
        return;
    }

    let coordinate = vec2<i32>(gid.xy);
    let pixel = textureLoad(source_texture, coordinate, 0);
    textureStore(destination_texture, coordinate, vec4<f32>(0.0, pixel.g, pixel.b, pixel.a));
}
