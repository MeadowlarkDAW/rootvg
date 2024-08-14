struct Globals {
    screen_to_clip_scale: vec2<f32>,
    scale_factor: f32,
}

struct InstanceUniforms {
    // We can't use a mat3x2<f32> directly because it does not have a stride
    // of 16, so decompose it like this.
    transform_0: f32,
    transform_1: f32,
    transform_2: f32,
    transform_3: f32,
    transform_4: f32,
    transform_5: f32,
    offset: vec2<f32>,
    do_transform: u32,
    snap_to_nearest_pixel: u32,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> instance_uniforms: InstanceUniforms;