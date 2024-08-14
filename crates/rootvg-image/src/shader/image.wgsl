struct Globals {
    screen_size_recip: vec2<f32>,
    scale_factor: f32,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var smp: sampler;

@group(1) @binding(0) var tex: texture_2d<f32>;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv_pos: vec2<f32>,
    @location(3) uv_size: vec2<f32>,
    @location(4) transform1: vec2<f32>,
    @location(5) transform2: vec2<f32>,
    @location(6) transform3: vec2<f32>,
    @location(7) do_transform: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv_pos: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let vertex_pos = vertex_position(input.vertex_index);

    var transformed_vertex_pos: vec2<f32> = vertex_pos.xy;
    if input.do_transform != 0 {
        transformed_vertex_pos =
            (mat3x2f(input.transform1, input.transform2, input.transform3)
            * vec3f(vertex_pos, 1.0)).xy;
    }

    let screen_pos: vec2<f32> =
        (input.pos + (transformed_vertex_pos * input.size))
        * globals.scale_factor;
    out.position = vec4<f32>(
        (screen_pos.x * globals.screen_size_recip.x) - 1.0,
        1.0 - (screen_pos.y * globals.screen_size_recip.y),
        0.0,
        1.0
    );

    out.uv_pos = input.uv_pos + (vertex_pos * input.uv_size);

    return out;
}

// Compute the normalized quad coordinates based on the vertex index.
fn vertex_position(vertex_index: u32) -> vec2<f32> {
    // #: 0 1 2 3 4 5
    // x: 1 1 0 0 0 1
    // y: 1 0 0 0 1 1
    return vec2<f32>((vec2(1u, 2u) + vertex_index) % vec2(6u) < vec2(3u));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(tex, smp, input.uv_pos);
}