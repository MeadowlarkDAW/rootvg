// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/wgpu/src/shader/quad/solid.wgsl
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

struct SolidVertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) color: vec4<f32>,
    @location(1) pos: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) border_color: vec4<f32>,
    @location(4) border_radius: vec4<f32>,
    @location(5) border_width: f32,
    @location(6) shadow_color: vec4<f32>,
    @location(7) shadow_offset: vec2<f32>,
    @location(8) shadow_blur_radius: f32,
}

struct SolidVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) border_color: vec4<f32>,
    @location(2) pos: vec2<f32>,
    @location(3) size: vec2<f32>,
    @location(4) border_radius: vec4<f32>,
    @location(5) border_width: f32,
    @location(6) shadow_color: vec4<f32>,
    @location(7) shadow_offset: vec2<f32>,
    @location(8) shadow_blur_radius: f32,
}

@vertex
fn solid_vs_main(input: SolidVertexInput) -> SolidVertexOutput {
    var out: SolidVertexOutput;

    var min_border_radius = min(input.size.x, input.size.y) * 0.5;
    var border_radius: vec4<f32> = vec4<f32>(
        min(input.border_radius.x, min_border_radius),
        min(input.border_radius.y, min_border_radius),
        min(input.border_radius.z, min_border_radius),
        min(input.border_radius.w, min_border_radius)
    );

    let screen_pos: vec2<f32> = input.pos + (vertex_position(input.vertex_index) * input.size);
    out.position = vec4<f32>(
        (screen_pos.x * globals.screen_to_clip_scale.x) - 1.0,
        1.0 - (screen_pos.y * globals.screen_to_clip_scale.y),
        0.0,
        1.0
    );

    out.color = input.color;
    out.border_color = input.border_color;
    out.pos = input.pos * globals.scale_factor;
    out.size = input.size * globals.scale_factor;
    out.border_radius = border_radius * globals.scale_factor;
    out.border_width = input.border_width * globals.scale_factor;
    out.shadow_color = input.shadow_color;
    out.shadow_offset = input.shadow_offset * globals.scale_factor;
    out.shadow_blur_radius = input.shadow_blur_radius * globals.scale_factor;

    return out;
}

@fragment
fn solid_fs_main(
    input: SolidVertexOutput
) -> @location(0) vec4<f32> {
    var mixed_color: vec4<f32> = input.color;

    var border_radius = select_border_radius(
        input.border_radius,
        input.position.xy,
        (input.pos + (input.size * 0.5)).xy
    );

    if (input.border_width > 0.0) {
        var internal_border: f32 = max(border_radius - input.border_width, 0.0);

        var internal_distance: f32 = distance_alg(
            input.position.xy,
            input.pos + vec2<f32>(input.border_width, input.border_width),
            input.size - vec2<f32>(input.border_width * 2.0, input.border_width * 2.0),
            internal_border
        );

        var border_mix: f32 = smoothstep(
            max(internal_border - 0.5, 0.0),
            internal_border + 0.5,
            internal_distance
        );

        mixed_color = mix(input.color, input.border_color, vec4<f32>(border_mix, border_mix, border_mix, border_mix));
    }

    var dist: f32 = distance_alg(
        vec2<f32>(input.position.x, input.position.y),
        input.pos,
        input.size,
        border_radius
    );

    var radius_alpha: f32 = 1.0 - smoothstep(
        max(border_radius - 0.5, 0.0),
        border_radius + 0.5,
        dist
    );

    let quad_color = vec4<f32>(mixed_color.x, mixed_color.y, mixed_color.z, mixed_color.w * radius_alpha);

    if input.shadow_color.a > 0.0 {
        let shadow_distance = rounded_box_sdf(input.position.xy - input.pos - input.shadow_offset - (input.size / 2.0), input.size / 2.0, border_radius);
        let shadow_alpha = 1.0 - smoothstep(-input.shadow_blur_radius, input.shadow_blur_radius, shadow_distance);
        let shadow_color = input.shadow_color;
        let base_color = select(
            vec4<f32>(shadow_color.x, shadow_color.y, shadow_color.z, 0.0),
            quad_color,
            quad_color.a > 0.0
        );

        return mix(base_color, shadow_color, (1.0 - radius_alpha) * shadow_alpha);
    } else {
        return quad_color;
    }
}