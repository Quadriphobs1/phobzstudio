//! Spectrum mountain visualization design.
//!
//! Filled polygon representing audio spectrum.

use super::{Design, DesignConfig, DesignParams, DesignType, Vertex};

/// Rendering context for spectrum mountain calculations.
struct MountainContext {
    width: f32,
    height: f32,
    beat_scale: f32,
    local_expand: f32,
}

impl MountainContext {
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

    /// Push a vertical slice (quad from baseline to spectrum value).
    #[allow(clippy::too_many_arguments)]
    fn push_slice(
        &self,
        vertices: &mut Vec<Vertex>,
        x1: f32,
        x2: f32,
        y_top1: f32,
        y_top2: f32,
        y_bottom: f32,
        value: f32,
        index: f32,
    ) {
        let positions = [
            self.to_ndc(x1, y_top1),   // top-left
            self.to_ndc(x2, y_top2),   // top-right
            self.to_ndc(x1, y_bottom), // bottom-left
            self.to_ndc(x2, y_bottom), // bottom-right
        ];

        let local = self.local_expand;
        // Use height-based local positions for gradient effect
        let local_positions = [
            [-local, -local], // top
            [local, -local],  // top
            [-local, local],  // bottom
            [local, local],   // bottom
        ];
        let indices = [0, 2, 1, 1, 2, 3]; // Two triangles

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

/// Filled polygon spectrum visualization (mountain/area chart).
pub struct SpectrumMountainDesign;

impl Design for SpectrumMountainDesign {
    fn design_type(&self) -> DesignType {
        DesignType::SpectrumMountain
    }

    fn generate_vertices(
        &self,
        spectrum: &[f32],
        config: &DesignConfig,
        params: &DesignParams,
    ) -> Vec<Vertex> {
        let params = match params {
            DesignParams::SpectrumMountain(p) => p,
            _ => return Vec::new(),
        };

        let point_count = spectrum.len().min(config.bar_count as usize);
        if point_count < 2 {
            return Vec::new();
        }

        let ctx = MountainContext::new(config);
        // Each slice = 6 vertices, we have (point_count - 1) slices
        let mut vertices = Vec::with_capacity((point_count - 1) * 6);

        let baseline = ctx.height * params.baseline;
        let max_height = ctx.height * (1.0 - params.baseline) * 0.9;

        // Apply smoothing for better visual
        let smoothed: Vec<f32> = if params.smoothing > 0.0 {
            smooth_spectrum(spectrum, point_count, params.smoothing)
        } else {
            spectrum.iter().take(point_count).copied().collect()
        };

        // Generate filled slices
        for i in 0..(point_count - 1) {
            let t1 = i as f32 / (point_count - 1) as f32;
            let t2 = (i + 1) as f32 / (point_count - 1) as f32;

            let x1 = t1 * ctx.width;
            let x2 = t2 * ctx.width;

            let v1 = smoothed[i].clamp(0.0, 1.0);
            let v2 = smoothed[i + 1].clamp(0.0, 1.0);

            let y_top1 = baseline - v1 * max_height * ctx.beat_scale;
            let y_top2 = baseline - v2 * max_height * ctx.beat_scale;

            // Handle mirror mode (reflect below baseline)
            if params.mirror {
                // Top half (above baseline going up)
                ctx.push_slice(
                    &mut vertices,
                    x1,
                    x2,
                    y_top1,
                    y_top2,
                    baseline,
                    (v1 + v2) * 0.5,
                    i as f32,
                );
                // Bottom half (below baseline going down)
                let y_bottom1 = baseline + v1 * max_height * ctx.beat_scale;
                let y_bottom2 = baseline + v2 * max_height * ctx.beat_scale;
                ctx.push_slice(
                    &mut vertices,
                    x1,
                    x2,
                    baseline,
                    baseline,
                    y_bottom1.max(y_bottom2),
                    (v1 + v2) * 0.5,
                    i as f32,
                );
            } else {
                ctx.push_slice(
                    &mut vertices,
                    x1,
                    x2,
                    y_top1,
                    y_top2,
                    baseline,
                    (v1 + v2) * 0.5,
                    i as f32,
                );
            }
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
