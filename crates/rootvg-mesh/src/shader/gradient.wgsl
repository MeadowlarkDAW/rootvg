// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/wgpu/src/shader/triangle/gradient.wgsl
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

struct GradientVertexInput {
    @location(0) position: vec2<f32>,
    @location(1) @interpolate(flat) colors_1: vec4<u32>,
    @location(2) @interpolate(flat) colors_2: vec4<u32>,
    @location(3) @interpolate(flat) offsets: vec2<u32>,
    @location(4) direction: vec4<f32>,
}

struct GradientVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) raw_position: vec2<f32>,
    @location(1) @interpolate(flat) colors_1: vec4<u32>,
    @location(2) @interpolate(flat) colors_2: vec4<u32>,
    @location(3) @interpolate(flat) offsets: vec2<u32>,
    @location(4) direction: vec4<f32>,
}

@vertex
fn gradient_vs_main(input: GradientVertexInput) -> GradientVertexOutput {
    var out: GradientVertexOutput;

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

    out.position = vec4<f32>(
        ((transformed_pos.x + instance_uniforms.offset.x) * globals.screen_to_clip_scale.x) - 1.0,
        1.0 - ((transformed_pos.y + instance_uniforms.offset.y) * globals.screen_to_clip_scale.y),
        0.0,
        1.0
    );

    out.raw_position = input.position;
    out.colors_1 = input.colors_1;
    out.colors_2 = input.colors_2;
    out.offsets = input.offsets;
    out.direction = input.direction;

    return out;
}

/// Returns the current interpolated color with a max 8-stop gradient
fn gradient(
    raw_position: vec2<f32>,
    direction: vec4<f32>,
    colors: array<vec4<f32>, 4>,
    offsets: vec4<f32>,
    last_index: i32
) -> vec4<f32> {
    let start = direction.xy;
    let end = direction.zw;

    let v1 = end - start;
    let v2 = raw_position - start;
    let unit = normalize(v1);
    let coord_offset = dot(unit, v2) / length(v1);

    //need to store these as a var to use dynamic indexing in a loop
    //this is already added to wgsl spec but not in wgpu yet
    var colors_arr = colors;
    var offsets_arr = offsets;

    var color: vec4<f32>;

    let noise_granularity: f32 = 0.3/255.0;

    for (var i: i32 = 0; i < last_index; i++) {
        let curr_offset = offsets_arr[i];
        let next_offset = offsets_arr[i+1];

        if (coord_offset <= offsets_arr[0]) {
            color = colors_arr[0];
        }

        if (curr_offset <= coord_offset && coord_offset <= next_offset) {
            let from_ = colors_arr[i];
            let to_ = colors_arr[i+1];
            let factor = smoothstep(curr_offset, next_offset, coord_offset);

            color = interpolate_color(from_, to_, factor);
        }

        if (coord_offset >= offsets_arr[last_index]) {
            color = colors_arr[last_index];
        }
    }

    return color + mix(-noise_granularity, noise_granularity, random(raw_position));
}

@fragment
fn gradient_fs_main(input: GradientVertexOutput) -> @location(0) vec4<f32> {
    let colors = array<vec4<f32>, 4>(
        unpack_u32(input.colors_1.xy),
        unpack_u32(input.colors_1.zw),
        unpack_u32(input.colors_2.xy),
        unpack_u32(input.colors_2.zw),
    );

    let offsets: vec4<f32> = unpack_u32(input.offsets);

    var last_index = 3;
    for (var i: i32 = 0; i <= 3; i++) {
        if (offsets[i] >= 1.0) {
            last_index = i;
            break;
        }
    }

    return gradient(input.raw_position, input.direction, colors, offsets, last_index);
}

fn unpack_u32(color: vec2<u32>) -> vec4<f32> {
    let rg: vec2<f32> = unpack2x16float(color.x);
    let ba: vec2<f32> = unpack2x16float(color.y);

    return vec4<f32>(rg.y, rg.x, ba.y, ba.x);
}

fn random(coords: vec2<f32>) -> f32 {
    return fract(sin(dot(coords, vec2(12.9898,78.233))) * 43758.5453);
}