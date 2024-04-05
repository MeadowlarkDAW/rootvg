struct SolidVertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct SolidVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn solid_vs_main(input: SolidVertexInput) -> SolidVertexOutput {
    var out: SolidVertexOutput;

    out.color = input.color;

    var transformed_pos: vec2<f32> = input.position.xy;
    if instance_uniforms.do_transform != 0 {
        transformed_pos = (instance_uniforms.transform * vec3f(input.position, 1.0)).xy;
    }

    out.position = vec4<f32>(
        ((transformed_pos.x + instance_uniforms.offset.x) * globals.screen_to_clip_scale.x) - 1.0,
        1.0 - ((transformed_pos.y + instance_uniforms.offset.y) * globals.screen_to_clip_scale.y),
        0.0,
        1.0
    );

    return out;
}

@fragment
fn solid_fs_main(input: SolidVertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}