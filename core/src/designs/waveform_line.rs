//! Waveform line visualization design.
//!
//! Classic oscilloscope-style line waveform connecting spectrum points.

use super::{Design, DesignConfig, DesignParams, DesignType, Vertex};

/// Rendering context for waveform line calculations.
struct LineContext {
    width: f32,
    height: f32,
    beat_scale: f32,
    local_expand: f32,
}

impl LineContext {
    fn new(config: &DesignConfig) -> Self {
        let glow_expand = if config.glow { 0.3 } else { 0.0 };
        Self {
            width: config.width as f32,
            height: config.height as f32,
            beat_scale: 1.0 + config.beat_intensity * 0.15,
            local_expand: 1.0 + glow_expand,
        }
    }

    #[inline]
    fn to_ndc(&self, x: f32, y: f32) -> [f32; 2] {
        [(x / self.width) * 2.0 - 1.0, 1.0 - (y / self.height) * 2.0]
    }

    /// Push a line segment as a quad (thick line).
    #[allow(clippy::too_many_arguments)]
    fn push_line_segment(
        &self,
        vertices: &mut Vec<Vertex>,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        half_width: f32,
        value: f32,
        index: f32,
    ) {
        // Calculate perpendicular direction for line thickness
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let nx = -dy / len * half_width;
        let ny = dx / len * half_width;

        let positions = [
            self.to_ndc(x1 + nx, y1 + ny), // start top
            self.to_ndc(x1 - nx, y1 - ny), // start bottom
            self.to_ndc(x2 + nx, y2 + ny), // end top
            self.to_ndc(x2 - nx, y2 - ny), // end bottom
        ];

        let local = self.local_expand;
        let local_positions = [
            [-local, -local],
            [-local, local],
            [local, -local],
            [local, local],
        ];
        let indices = [0, 1, 2, 2, 1, 3]; // Two triangles

        for &idx in &indices {
            vertices.push(Vertex {
                position: positions[idx],
                local_pos: local_positions[idx],
                bar_height: value,
                bar_index: index,
            });
        }
    }
}

/// Classic oscilloscope-style waveform line.
pub struct WaveformLineDesign;

impl Design for WaveformLineDesign {
    fn design_type(&self) -> DesignType {
        DesignType::WaveformLine
    }

    fn generate_vertices(
        &self,
        spectrum: &[f32],
        config: &DesignConfig,
        params: &DesignParams,
    ) -> Vec<Vertex> {
        let params = match params {
            DesignParams::WaveformLine(p) => p,
            _ => return Vec::new(),
        };

        let point_count = spectrum.len().min(config.bar_count as usize);
        if point_count < 2 {
            return Vec::new();
        }

        let ctx = LineContext::new(config);
        // Each line segment = 6 vertices, we have (point_count - 1) segments
        let mut vertices = Vec::with_capacity((point_count - 1) * 6);

        let half_width = params.line_width * 0.5 * ctx.local_expand;
        let center_y = ctx.height * 0.5;
        let amplitude = ctx.height * 0.4 * ctx.beat_scale;

        // Generate smoothed points if smoothing is enabled
        let smoothed: Vec<f32> = if params.smoothing > 0.0 {
            smooth_spectrum(spectrum, point_count, params.smoothing)
        } else {
            spectrum.iter().take(point_count).copied().collect()
        };

        // Generate line segments
        for i in 0..(point_count - 1) {
            let t1 = i as f32 / (point_count - 1) as f32;
            let t2 = (i + 1) as f32 / (point_count - 1) as f32;

            let x1 = t1 * ctx.width;
            let x2 = t2 * ctx.width;

            let v1 = smoothed[i].clamp(0.0, 1.0);
            let v2 = smoothed[i + 1].clamp(0.0, 1.0);

            // Mirror mode: oscillate above and below center
            let y1 = if params.mirror {
                center_y + (v1 - 0.5) * amplitude * 2.0
            } else {
                ctx.height - v1 * amplitude - ctx.height * 0.1
            };

            let y2 = if params.mirror {
                center_y + (v2 - 0.5) * amplitude * 2.0
            } else {
                ctx.height - v2 * amplitude - ctx.height * 0.1
            };

            let avg_value = (v1 + v2) * 0.5;
            ctx.push_line_segment(
                &mut vertices,
                x1,
                y1,
                x2,
                y2,
                half_width,
                avg_value,
                i as f32,
            );
        }

        vertices
    }
}

/// Apply simple moving average smoothing to spectrum.
fn smooth_spectrum(spectrum: &[f32], count: usize, smoothing: f32) -> Vec<f32> {
    let window = ((smoothing * 5.0) as usize).max(1).min(count / 2);
    let mut result = Vec::with_capacity(count);

    for i in 0..count {
        if i >= spectrum.len() {
            result.push(0.0);
            continue;
        }

        let start = i.saturating_sub(window);
        let end = (i + window + 1).min(spectrum.len());
        let sum: f32 = spectrum[start..end].iter().sum();
        result.push(sum / (end - start) as f32);
    }

    result
}
