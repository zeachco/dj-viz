// Feedback buffer shader for trail effect
// Samples previous frame, applies fade and optional scale transform

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct Uniforms {
    fade: f32,
    scale: f32,
    _padding: vec2<f32>,
};

@group(0) @binding(0)
var t_prev: texture_2d<f32>;

@group(0) @binding(1)
var s_prev: sampler;

@group(0) @binding(2)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Apply scale transform (zoom toward/away from center)
    let centered = in.tex_coords - vec2<f32>(0.5, 0.5);
    let scaled = centered * uniforms.scale;
    let uv = scaled + vec2<f32>(0.5, 0.5);

    // Clamp UVs to avoid sampling outside texture
    let clamped_uv = clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0));

    // Sample previous frame
    var color = textureSample(t_prev, s_prev, clamped_uv);

    // Apply fade (darken towards black)
    color = vec4<f32>(color.rgb * uniforms.fade, color.a * uniforms.fade);

    return color;
}
