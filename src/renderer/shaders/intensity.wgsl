// Intensity effects shader for emotional music visualization
// Applies vignette, color warmth, saturation boost, and pulse based on intensity

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct IntensityUniforms {
    intensity: f32,      // Overall intensity (0-1)
    momentum: f32,       // Intensity momentum (for pulse timing)
    bass: f32,           // Bass energy (for extra warmth)
    time: f32,           // Time for subtle animations
};

@group(0) @binding(0)
var t_input: texture_2d<f32>;

@group(0) @binding(1)
var s_sampler: sampler;

@group(0) @binding(2)
var<uniform> uniforms: IntensityUniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    return out;
}

// Convert RGB to HSL
fn rgb_to_hsl(c: vec3<f32>) -> vec3<f32> {
    let cmax = max(max(c.r, c.g), c.b);
    let cmin = min(min(c.r, c.g), c.b);
    let delta = cmax - cmin;

    var h = 0.0;
    var s = 0.0;
    let l = (cmax + cmin) / 2.0;

    if (delta > 0.0) {
        s = delta / (1.0 - abs(2.0 * l - 1.0));

        if (cmax == c.r) {
            h = (c.g - c.b) / delta;
            if (c.g < c.b) { h = h + 6.0; }
        } else if (cmax == c.g) {
            h = (c.b - c.r) / delta + 2.0;
        } else {
            h = (c.r - c.g) / delta + 4.0;
        }
        h = h / 6.0;
    }

    return vec3<f32>(h, s, l);
}

// Convert HSL to RGB
fn hsl_to_rgb(hsl: vec3<f32>) -> vec3<f32> {
    let h = hsl.x;
    let s = hsl.y;
    let l = hsl.z;

    if (s == 0.0) {
        return vec3<f32>(l, l, l);
    }

    let q = select(l + s - l * s, l * (1.0 + s), l < 0.5);
    let p = 2.0 * l - q;

    let r = hue_to_rgb(p, q, h + 1.0/3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0/3.0);

    return vec3<f32>(r, g, b);
}

fn hue_to_rgb(p: f32, q: f32, t_in: f32) -> f32 {
    var t = t_in;
    if (t < 0.0) { t = t + 1.0; }
    if (t > 1.0) { t = t - 1.0; }
    if (t < 1.0/6.0) { return p + (q - p) * 6.0 * t; }
    if (t < 1.0/2.0) { return q; }
    if (t < 2.0/3.0) { return p + (q - p) * (2.0/3.0 - t) * 6.0; }
    return p;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;
    var color = textureSample(t_input, s_sampler, uv);

    let intensity = uniforms.intensity;
    let momentum = uniforms.momentum;
    let bass = uniforms.bass;

    // === VIGNETTE ===
    // Stronger vignette as intensity rises - creates focus and immersion
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(uv, center);
    let vignette_strength = intensity * 0.6; // Max 60% vignette at full intensity
    let vignette_radius = 1.0 - intensity * 0.3; // Tighter radius with intensity
    let vignette = 1.0 - smoothstep(vignette_radius * 0.3, vignette_radius, dist) * vignette_strength;

    // === COLOR WARMTH ===
    // Shift toward warm colors (reds/oranges) as intensity rises
    // Especially strong with bass
    let warmth = intensity * 0.4 + bass * 0.2;
    let warm_tint = vec3<f32>(1.0 + warmth * 0.15, 1.0 + warmth * 0.05, 1.0 - warmth * 0.1);
    color = vec4<f32>(color.rgb * warm_tint, color.a);

    // === SATURATION BOOST ===
    // Increase saturation as intensity rises - more vivid, emotional colors
    let hsl = rgb_to_hsl(color.rgb);
    let sat_boost = 1.0 + intensity * 0.4; // Up to 40% more saturation
    let boosted_sat = clamp(hsl.y * sat_boost, 0.0, 1.0);
    color = vec4<f32>(hsl_to_rgb(vec3<f32>(hsl.x, boosted_sat, hsl.z)), color.a);

    // === PULSE EFFECT ===
    // Subtle brightness pulse synced to momentum
    let pulse_freq = 2.0 + momentum * 4.0; // Faster pulse with higher momentum
    let pulse = sin(uniforms.time * pulse_freq) * 0.5 + 0.5;
    let pulse_strength = momentum * 0.08; // Subtle pulse strength
    let brightness_mod = 1.0 + pulse * pulse_strength;
    color = vec4<f32>(color.rgb * brightness_mod, color.a);

    // === CONTRAST BOOST ===
    // Slight contrast increase with intensity for more dramatic feel
    let contrast = 1.0 + intensity * 0.15;
    let contrasted = (color.rgb - 0.5) * contrast + 0.5;
    color = vec4<f32>(clamp(contrasted, vec3<f32>(0.0), vec3<f32>(1.0)), color.a);

    // Apply vignette
    color = vec4<f32>(color.rgb * vignette, color.a);

    return color;
}
