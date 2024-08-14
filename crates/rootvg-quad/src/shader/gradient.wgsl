// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/wgpu/src/shader/quad/gradient.wgsl
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

struct GradientVertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) @interpolate(flat) colors_1: vec4<u32>,
    @location(1) @interpolate(flat) colors_2: vec4<u32>,
    @location(2) @interpolate(flat) offsets: vec2<u32>,
    @location(3) direction: vec4<f32>,
    @location(4) pos: vec2<f32>,
    @location(5) size: vec2<f32>,
    @location(6) border_color: vec4<f32>,
    @location(7) border_radius: vec4<f32>,
    @location(8) border_width: f32,
    @location(9) flags: u32,
}

struct GradientVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) @interpolate(flat) colors_1: vec4<u32>,
    @location(2) @interpolate(flat) colors_2: vec4<u32>,
    @location(3) @interpolate(flat) offsets: vec2<u32>,
    @location(4) direction: vec4<f32>,
    @location(5) pos: vec2<f32>,
    @location(6) size: vec2<f32>,
    @location(7) border_color: vec4<f32>,
    @location(8) border_radius: vec4<f32>,
    @location(9) border_width: f32,
}

@vertex
fn gradient_vs_main(input: GradientVertexInput) -> GradientVertexOutput {
    var out: GradientVertexOutput;

    var min_border_radius = min(input.size.x, input.size.y) * 0.5;
    var border_radius: vec4<f32> = vec4<f32>(
        min(input.border_radius.x, min_border_radius),
        min(input.border_radius.y, min_border_radius),
        min(input.border_radius.z, min_border_radius),
        min(input.border_radius.w, min_border_radius)
    );

    var screen_pos: vec2<f32> =
        (input.pos + (vertex_position(input.vertex_index) * input.size))
        * globals.scale_factor;

    out.colors_1 = input.colors_1;
    out.colors_2 = input.colors_2;
    out.offsets = input.offsets;
    out.direction = input.direction * globals.scale_factor;
    out.pos = input.pos * globals.scale_factor;
    out.size = input.size * globals.scale_factor;
    out.border_color = input.border_color;
    out.border_radius = border_radius * globals.scale_factor;
    out.border_width = input.border_width * globals.scale_factor;

    // Snap edges to nearest physical pixel.
    if (input.flags & 1u) > 0 {
        let snapped_end_pos = round((input.pos + input.size) * globals.scale_factor);

        screen_pos = round(screen_pos);
        out.pos = round(out.pos);
        out.size = snapped_end_pos - out.pos;
    }
    // Snap border width to nearest physical pixel.
    if (input.flags & 2u) > 0 {
        out.border_width = round(out.border_width);
    }

    out.position = vec4<f32>(
        (screen_pos.x * globals.screen_size_recip.x) - 1.0,
        1.0 - (screen_pos.y * globals.screen_size_recip.y),
        0.0,
        1.0
    );

    return out;
}

fn random(coords: vec2<f32>) -> f32 {
    return fract(sin(dot(coords, vec2(12.9898,78.233))) * 43758.5453);
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

    // TODO could just pass this in to the shader but is probably more performant to just check it here
    var last_index = 3;
    for (var i: i32 = 0; i <= 3; i++) {
        if (offsets[i] > 1.0) {
            last_index = i - 1;
            break;
        }
    }

    var mixed_color: vec4<f32> = gradient(input.position.xy, input.direction, colors, offsets, last_index);

    var border_radius = select_border_radius(
        input.border_radius,
        input.position.xy,
        (input.pos + input.size * 0.5).xy
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

        mixed_color = mix(mixed_color, input.border_color, vec4<f32>(border_mix, border_mix, border_mix, border_mix));
    }

    var dist: f32 = distance_alg(
        input.position.xy,
        input.pos,
        input.size,
        border_radius
    );

    var radius_alpha: f32 = 1.0 - smoothstep(
        max(border_radius - 0.5, 0.0),
        border_radius + 0.5,
        dist);

    return vec4<f32>(mixed_color.x, mixed_color.y, mixed_color.z, mixed_color.w * radius_alpha);
}

fn unpack_u32(color: vec2<u32>) -> vec4<f32> {
    let rg: vec2<f32> = unpack2x16float(color.x);
    let ba: vec2<f32> = unpack2x16float(color.y);

    return vec4<f32>(rg.y, rg.x, ba.y, ba.x);
}