struct Globals {
    screen_to_clip_scale: vec2<f32>,
    scale_factor: f32,
}

struct InstanceUniforms {
    offset: vec2<f32>,
    transform: mat3x2<f32>,
    do_transform: u32,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> instance_uniforms: InstanceUniforms;