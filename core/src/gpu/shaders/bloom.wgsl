// Bloom extraction and composition shader.
// Two entry points: extract bright areas, composite bloom with scene.

struct BloomUniforms {
    threshold: f32,
    intensity: f32,
    beat_intensity: f32,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: BloomUniforms;

@group(0) @binding(1)
var scene_texture: texture_2d<f32>;

@group(0) @binding(2)
var bloom_texture: texture_2d<f32>;

@group(0) @binding(3)
var input_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle vertex shader (shared)
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;

    let x = f32(i32(vertex_index & 1u) * 4 - 1);
    let y = f32(i32(vertex_index >> 1u) * 4 - 1);

    output.position = vec4<f32>(x, y, 0.0, 1.0);
    output.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);

    return output;
}

// Extract bright areas for bloom
@fragment
fn fs_extract(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(scene_texture, input_sampler, input.uv);

    // Calculate luminance
    let luminance = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));

    // Soft threshold extraction with beat-reactive threshold adjustment
    let adjusted_threshold = uniforms.threshold * (1.0 - uniforms.beat_intensity * 0.3);
    let soft_threshold = smoothstep(adjusted_threshold, adjusted_threshold + 0.2, luminance);

    // Extract bright areas with original color
    let extracted = color.rgb * soft_threshold;

    return vec4<f32>(extracted, color.a * soft_threshold);
}

// Composite bloom with original scene
@fragment
fn fs_composite(input: VertexOutput) -> @location(0) vec4<f32> {
    let scene = textureSample(scene_texture, input_sampler, input.uv);
    let bloom = textureSample(bloom_texture, input_sampler, input.uv);

    // Beat-reactive bloom intensity
    let effective_intensity = uniforms.intensity * (1.0 + uniforms.beat_intensity * 0.5);

    // Additive blend with intensity control
    var result = scene.rgb + bloom.rgb * effective_intensity;

    // Soft clamp to prevent harsh cutoffs
    result = result / (result + vec3<f32>(1.0)) * 2.0; // Reinhard-style tonemap
    result = clamp(result, vec3<f32>(0.0), vec3<f32>(1.0));

    // Preserve alpha from scene
    let alpha = max(scene.a, bloom.a * effective_intensity);

    return vec4<f32>(result, clamp(alpha, 0.0, 1.0));
}
