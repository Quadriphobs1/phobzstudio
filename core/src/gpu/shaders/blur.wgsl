// Two-pass separable Gaussian blur shader.
// Uses a 9-tap kernel for efficient blur.

struct BlurUniforms {
    direction: vec2<f32>,  // (1,0) for horizontal, (0,1) for vertical
    texel_size: vec2<f32>, // 1.0 / texture_dimensions
}

@group(0) @binding(0)
var<uniform> uniforms: BlurUniforms;

@group(0) @binding(1)
var input_texture: texture_2d<f32>;

@group(0) @binding(2)
var input_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle vertex shader
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;

    // Generate fullscreen triangle
    let x = f32(i32(vertex_index & 1u) * 4 - 1);
    let y = f32(i32(vertex_index >> 1u) * 4 - 1);

    output.position = vec4<f32>(x, y, 0.0, 1.0);
    output.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);

    return output;
}

// 9-tap Gaussian blur weights (sigma ~= 2.0)
const WEIGHTS: array<f32, 5> = array<f32, 5>(
    0.227027027,
    0.1945945946,
    0.1216216216,
    0.0540540541,
    0.0162162162
);

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let offset = uniforms.direction * uniforms.texel_size;

    // Center sample
    var result = textureSample(input_texture, input_sampler, input.uv) * WEIGHTS[0];

    // Symmetric taps (both directions)
    for (var i = 1; i < 5; i++) {
        let sample_offset = offset * f32(i);
        result += textureSample(input_texture, input_sampler, input.uv + sample_offset) * WEIGHTS[i];
        result += textureSample(input_texture, input_sampler, input.uv - sample_offset) * WEIGHTS[i];
    }

    return result;
}
