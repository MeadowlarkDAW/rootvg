struct VertUniforms {
    /// The reciprical of the view size times two
    view_size_recip_2: vec2f,
}

struct FragUniforms {
    scissor_mat: mat3x4<f32>,
    paint_mat: mat3x4<f32>,
    inner_color: vec4f,
    out_color: vec4f,
    scissor_ext: vec2f,
    scissor_scale: vec2f,
    extent: vec2f,
    radius: f32,
    feather: f32,
    stroke_mult: f32,
    stroke_thr: f32,
    text_type: u32,
    type_: u32,
}

@group(0) @binding(0) var<uniform> vert_uniforms: VertUniforms;
@group(1) @binding(0) var<uniform> frag_uniforms: FragUniforms;

// -----------------------------------------------------------------------------------------------

struct VertexInput {
    @location(0) position: vec2f,
    @location(1) tcoord: vec2f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) ftcoord: vec2f,
    @location(1) fpos: vec2f,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.position = vec4<f32>(
        (input.position.x * vert_uniforms.view_size_recip_2.x) - 1.0,
        (1.0 - input.position.y * vert_uniforms.view_size_recip_2).y,
        0.0,
        1.0
    );

    out.ftcoord = input.tcoord;
    out.fpos = input.position;

    return out;
}

// -----------------------------------------------------------------------------------------------

const SHADER_TYPE_COLOR: u32 = 0;
const SHADER_TYPE_GRADIENT: u32 = 1;
const SHADER_TYPE_IMAGE: u32 = 2;
const SHADER_TYPE_STENCIL: u32 = 3;
const SHADER_TYPE_IMAGE_GRADIENT: u32 = 4;
const SHADER_TYPE_FILTER_IMAGE: u32 = 5;
const TEXTURE_COPY_UNCLIPPED: u32 = 6;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var result: vec4f;

    var stroke_alpha = 1.0;
    if EDGE_AA {
        if frag_uniforms.type_ != TEXTURE_COPY_UNCLIPPED {
            // Stroke - from [0..1] to clipped pyramid, where the slope is 1px
            stroke_alpha = min(1.0, (1.0-abs(input.ftcoord.x*2.0-1.0))*frag_uniforms.stroke_mult)
                * min(1.0, input.ftcoord.y);
            if stroke_alpha < frag_uniforms.stroke_thr {
                discard;
            }
        }
    };

    if frag_uniforms.type_ == SHADER_TYPE_COLOR {
        // Plain color fill;
        result = frag_uniforms.inner_color;
    } else if frag_uniforms.type_ == SHADER_TYPE_GRADIENT {
        result = render_gradient(input.fpos);
    } else if frag_uniforms.type_ == SHADER_TYPE_IMAGE {
        // todo
        discard;
    } else if frag_uniforms.type_ == SHADER_TYPE_STENCIL {
        result = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else if frag_uniforms.type_ == SHADER_TYPE_IMAGE_GRADIENT {
        // todo
        discard;
    } else if frag_uniforms.type_ == SHADER_TYPE_FILTER_IMAGE {
        // todo
        discard;
    } else if frag_uniforms.type_ == TEXTURE_COPY_UNCLIPPED {
        // todo
        discard;
    }

    let scissor = scissor_mask(input.fpos);

    if frag_uniforms.type_ != SHADER_TYPE_STENCIL && frag_uniforms.type_ != SHADER_TYPE_FILTER_IMAGE {
        // Combine alpha
        result *= stroke_alpha * scissor;
    }

    return result;
}

// Scissoring
fn scissor_mask(p: vec2f) -> f32 {
    var sc = (abs((frag_uniforms.scissor_mat * vec3f(p, 1.0)).xy) - frag_uniforms.scissor_ext);
	sc = vec2f(0.5, 0.5) - sc * frag_uniforms.scissor_scale;
	return clamp(sc.x, 0.0, 1.0) * clamp(sc.y, 0.0, 1.0);
}

fn sd_round_rect(pt: vec2f, ext: vec2f, rad: f32) -> f32 {
    let ext2 = ext - vec2<f32>(rad, rad);
    let d = abs(pt) - ext2;
    return min(max(d.x,d.y), 0.0) + length(max(d, vec2<f32>(0.0, 0.0))) - rad;
}

fn render_gradient(fpos: vec2f) -> vec4f {
    // Calculate gradient color using box gradient
    let pt = (frag_uniforms.paint_mat * vec3f(fpos, 1.0)).xy;

    let d = clamp(
        (sd_round_rect(pt, frag_uniforms.extent, frag_uniforms.radius)
            + frag_uniforms.feather * 0.5) / frag_uniforms.feather,
        0.0,
        1.0
    );
    return mix(frag_uniforms.inner_color, frag_uniforms.out_color, d);
}