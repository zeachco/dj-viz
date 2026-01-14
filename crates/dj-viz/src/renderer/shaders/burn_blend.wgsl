// Blend shader for compositing overlay textures
// Uses screen blend: result = 1 - (1 - base) * (1 - blend)

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@group(0) @binding(0)
var t_base: texture_2d<f32>;

@group(0) @binding(1)
var t_overlay: texture_2d<f32>;

@group(0) @binding(2)
var s_sampler: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    return out;
}

// Screen blend - additive-like, good for combining visualizations
fn screen_blend(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    return 1.0 - (1.0 - base) * (1.0 - blend);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSample(t_base, s_sampler, in.tex_coords);
    let overlay = textureSample(t_overlay, s_sampler, in.tex_coords);

    // Screen blend the overlay onto the base
    let blended = screen_blend(base.rgb, overlay.rgb);

    return vec4<f32>(blended, max(base.a, overlay.a));
}
