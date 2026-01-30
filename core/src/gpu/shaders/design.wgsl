// Universal design shader.
// Works with pre-computed vertex positions from any design type.

struct Uniforms {
    color: vec4<f32>,       // rgb + unused alpha
    beat_intensity: f32,
    glow_enabled: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) local_pos: vec2<f32>,
    @location(2) bar_height: f32,
    @location(3) bar_index: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) bar_height: f32,
    @location(2) beat_intensity: f32,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.local_pos = input.local_pos;
    output.bar_height = input.bar_height;
    output.beat_intensity = uniforms.beat_intensity;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let glow_on = uniforms.glow_enabled > 0.5;

    // Calculate distance from bar edge for glow effect
    let dist_x = abs(input.local_pos.x) - 1.0;
    let dist_y = abs(input.local_pos.y) - 1.0;
    let dist = max(max(dist_x, dist_y), 0.0);

    // Core bar (inside the original bounds)
    let inside_bar = dist_x <= 0.0 && dist_y <= 0.0;

    // Discard glow pixels when glow is disabled
    if !inside_bar && !glow_on {
        discard;
    }

    // Base color
    var color = uniforms.color.rgb;

    // Height-based brightness (louder = brighter)
    let height_boost = 0.7 + input.bar_height * 0.3;
    color = color * height_boost;

    // Beat pulse effect - brighten on beat
    let beat_pulse = 1.0 + input.beat_intensity * 0.5;
    color = color * beat_pulse;

    // Subtle color shift on beat (towards white)
    let beat_white = input.beat_intensity * 0.25;
    color = mix(color, vec3<f32>(1.0), beat_white);

    // Calculate alpha based on position
    var alpha: f32;

    if inside_bar {
        // Inside the bar - full opacity with edge softening
        let edge_dist = max(dist_x, dist_y);
        let edge_soft = smoothstep(-0.1, 0.0, edge_dist);
        alpha = 1.0 - edge_soft * 0.1;

        // Inner gradient for depth
        let inner_x = 1.0 - abs(input.local_pos.x);
        let inner_y = 1.0 - abs(input.local_pos.y);
        let inner_bright = 0.85 + min(inner_x, inner_y) * 0.15;
        color = color * inner_bright;
    } else {
        // Glow region - exponential falloff
        let glow_falloff = exp(-dist * 8.0);
        alpha = glow_falloff * 0.6;

        // Glow is slightly more saturated
        let glow_boost = 1.0 + input.beat_intensity * 0.3;
        color = color * glow_boost;
    }

    // Beat-reactive glow intensity (only when glow is enabled)
    if glow_on {
        let glow_intensity = 1.0 + input.beat_intensity * 0.5;
        alpha = alpha * glow_intensity;
    }

    // Clamp color to valid range
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    alpha = clamp(alpha, 0.0, 1.0);

    return vec4<f32>(color, alpha);
}
