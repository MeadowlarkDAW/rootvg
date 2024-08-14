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
        let transform = mat3x2<f32>(
            instance_uniforms.transform_0,
            instance_uniforms.transform_1,
            instance_uniforms.transform_2,
            instance_uniforms.transform_3,
            instance_uniforms.transform_4,
            instance_uniforms.transform_5,
        );

        transformed_pos = (transform * vec3f(input.position, 1.0)).xy;
    }

    var screen_pos = (transformed_pos + instance_uniforms.offset) * globals.scale_factor;

    if instance_uniforms.snap_to_nearest_pixel != 0 {
        screen_pos = round(screen_pos);
    }

    out.position = vec4<f32>(
        (screen_pos.x * globals.screen_size_recip.x) - 1.0,
        1.0 - (screen_pos.y * globals.screen_size_recip.y),
        0.0,
        1.0
    );

    return out;
}

@fragment
fn solid_fs_main(input: SolidVertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}