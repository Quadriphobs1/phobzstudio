// Waveform visualization shader.
// Renders bars representing audio amplitude with glow effects.
// Supports both horizontal (16:9) and vertical (9:16) layouts.

struct Uniforms {
    width: f32,
    height: f32,
    bar_count: f32,
    beat_intensity: f32,
    color: vec3<f32>,
    // Layout: 0.0 = horizontal bars, 1.0 = vertical bars
    layout_vertical: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) bar_height: f32,
    @location(1) bar_index: f32,
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) bar_height: f32,
    @location(2) beat_intensity: f32,
    @location(3) local_pos: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    let is_vertical = uniforms.layout_vertical > 0.5;

    // Beat-reactive scale effect
    let beat_scale = 1.0 + uniforms.beat_intensity * 0.15;

    // Glow expansion (render slightly larger quad for glow effect)
    let glow_expand = 0.3; // 30% expansion for glow

    var x: f32;
    var y: f32;
    var u: f32;
    var v: f32;
    var local_x: f32;
    var local_y: f32;

    if is_vertical {
        // Vertical layout: bars arranged vertically, extending horizontally
        let bar_height_px = uniforms.height / uniforms.bar_count;
        let gap = bar_height_px * 0.1;
        let actual_bar_height = bar_height_px - gap;
        let expanded_bar_height = actual_bar_height * (1.0 + glow_expand);

        // Bar position (from bottom)
        let bar_y = uniforms.height - (input.bar_index + 1.0) * bar_height_px + gap * 0.5;
        let center_bar_y = bar_y + actual_bar_height * 0.5;

        // Bar width scaled by amplitude, centered horizontally
        let scaled_width = input.bar_height * uniforms.width * 0.8 * beat_scale;
        let half_width = scaled_width * 0.5;
        let expanded_half_width = half_width * (1.0 + glow_expand);
        let center_x = uniforms.width * 0.5;

        switch input.vertex_index {
            case 0u: {
                x = center_x - expanded_half_width;
                y = center_bar_y - expanded_bar_height * 0.5;
                u = 0.0;
                v = 0.0;
                local_x = -1.0 - glow_expand;
                local_y = -1.0 - glow_expand;
            }
            case 1u: {
                x = center_x - expanded_half_width;
                y = center_bar_y + expanded_bar_height * 0.5;
                u = 0.0;
                v = 1.0;
                local_x = -1.0 - glow_expand;
                local_y = 1.0 + glow_expand;
            }
            case 2u: {
                x = center_x + expanded_half_width;
                y = center_bar_y - expanded_bar_height * 0.5;
                u = 1.0;
                v = 0.0;
                local_x = 1.0 + glow_expand;
                local_y = -1.0 - glow_expand;
            }
            case 3u: {
                x = center_x + expanded_half_width;
                y = center_bar_y + expanded_bar_height * 0.5;
                u = 1.0;
                v = 1.0;
                local_x = 1.0 + glow_expand;
                local_y = 1.0 + glow_expand;
            }
            default: {
                x = 0.0;
                y = 0.0;
                u = 0.0;
                v = 0.0;
                local_x = 0.0;
                local_y = 0.0;
            }
        }
    } else {
        // Horizontal layout: bars arranged horizontally, extending vertically
        let bar_width = uniforms.width / uniforms.bar_count;
        let gap = bar_width * 0.1;
        let actual_bar_width = bar_width - gap;
        let expanded_bar_width = actual_bar_width * (1.0 + glow_expand);

        // Bar position
        let bar_x = input.bar_index * bar_width + gap * 0.5;
        let center_bar_x = bar_x + actual_bar_width * 0.5;

        // Bar height scaled to viewport, centered vertically
        let scaled_height = input.bar_height * uniforms.height * 0.8 * beat_scale;
        let half_height = scaled_height * 0.5;
        let expanded_half_height = half_height * (1.0 + glow_expand);
        let center_y = uniforms.height * 0.5;

        switch input.vertex_index {
            case 0u: {
                x = center_bar_x - expanded_bar_width * 0.5;
                y = center_y - expanded_half_height;
                u = 0.0;
                v = 0.0;
                local_x = -1.0 - glow_expand;
                local_y = -1.0 - glow_expand;
            }
            case 1u: {
                x = center_bar_x - expanded_bar_width * 0.5;
                y = center_y + expanded_half_height;
                u = 0.0;
                v = 1.0;
                local_x = -1.0 - glow_expand;
                local_y = 1.0 + glow_expand;
            }
            case 2u: {
                x = center_bar_x + expanded_bar_width * 0.5;
                y = center_y - expanded_half_height;
                u = 1.0;
                v = 0.0;
                local_x = 1.0 + glow_expand;
                local_y = -1.0 - glow_expand;
            }
            case 3u: {
                x = center_bar_x + expanded_bar_width * 0.5;
                y = center_y + expanded_half_height;
                u = 1.0;
                v = 1.0;
                local_x = 1.0 + glow_expand;
                local_y = 1.0 + glow_expand;
            }
            default: {
                x = 0.0;
                y = 0.0;
                u = 0.0;
                v = 0.0;
                local_x = 0.0;
                local_y = 0.0;
            }
        }
    }

    // Convert to NDC (-1 to 1)
    let ndc_x = (x / uniforms.width) * 2.0 - 1.0;
    let ndc_y = 1.0 - (y / uniforms.height) * 2.0; // Flip Y

    output.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    output.uv = vec2<f32>(u, v);
    output.bar_height = input.bar_height;
    output.beat_intensity = uniforms.beat_intensity;
    output.local_pos = vec2<f32>(local_x, local_y);

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate distance from bar edge for glow effect
    let dist_x = abs(input.local_pos.x) - 1.0;
    let dist_y = abs(input.local_pos.y) - 1.0;
    let dist = max(max(dist_x, dist_y), 0.0);

    // Core bar (inside the original bounds)
    let inside_bar = dist_x <= 0.0 && dist_y <= 0.0;

    // Base color
    var color = uniforms.color;

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

    // Beat-reactive glow intensity
    let glow_intensity = 1.0 + input.beat_intensity * 0.5;
    alpha = alpha * glow_intensity;

    // Clamp color to valid range
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    alpha = clamp(alpha, 0.0, 1.0);

    return vec4<f32>(color, alpha);
}
