//! Circular visualization designs.

use super::{Design, DesignConfig, DesignParams, DesignType, Vertex};
use std::f32::consts::PI;

/// Shared context for circular rendering calculations.
struct CircularContext {
    center_x: f32,
    center_y: f32,
    min_dim: f32,
    width: f32,
    height: f32,
    glow_expand: f32,
    beat_scale: f32,
    local_expand: f32,
}

impl CircularContext {
    fn new(config: &DesignConfig) -> Self {
        let width = config.width as f32;
        let height = config.height as f32;
        let glow_expand = if config.glow { 0.3 } else { 0.0 };

        Self {
            center_x: width * 0.5,
            center_y: height * 0.5,
            min_dim: width.min(height),
            width,
            height,
            glow_expand,
            beat_scale: 1.0 + config.beat_intensity * 0.15,
            local_expand: 1.0 + glow_expand,
        }
    }

    #[inline]
    fn to_ndc(&self, x: f32, y: f32) -> [f32; 2] {
        [(x / self.width) * 2.0 - 1.0, 1.0 - (y / self.height) * 2.0]
    }

    /// Generate a radial quad from angle and radii.
    #[allow(clippy::too_many_arguments)]
    fn push_radial_quad(
        &self,
        vertices: &mut Vec<Vertex>,
        angle: f32,
        half_angle: f32,
        inner_r: f32,
        outer_r: f32,
        bar_height: f32,
        bar_index: f32,
    ) {
        let (sin_l, cos_l) = (angle - half_angle).sin_cos();
        let (sin_r, cos_r) = (angle + half_angle).sin_cos();

        // Glow expansion on radii
        let inner_r = inner_r * (1.0 - self.glow_expand * 0.3);
        let outer_r = outer_r * (1.0 + self.glow_expand * 0.3);

        // Calculate corner positions
        let positions = [
            self.to_ndc(self.center_x + cos_l * inner_r, self.center_y + sin_l * inner_r),
            self.to_ndc(self.center_x + cos_r * inner_r, self.center_y + sin_r * inner_r),
            self.to_ndc(self.center_x + cos_l * outer_r, self.center_y + sin_l * outer_r),
            self.to_ndc(self.center_x + cos_r * outer_r, self.center_y + sin_r * outer_r),
        ];

        let local = self.local_expand;
        let local_positions = [
            [-local, -local],
            [local, -local],
            [-local, local],
            [local, local],
        ];

        // Triangle 1: inner-left, outer-left, inner-right
        // Triangle 2: inner-right, outer-left, outer-right
        let indices = [0, 2, 1, 1, 2, 3];

        for &idx in &indices {
            vertices.push(Vertex {
                position: positions[idx],
                local_pos: local_positions[idx],
                bar_height,
                bar_index,
            });
        }
    }
}

/// Bars emanating outward from center in a radial pattern.
pub struct CircularRadialDesign;

impl Design for CircularRadialDesign {
    fn design_type(&self) -> DesignType {
        DesignType::CircularRadial
    }

    fn generate_vertices(
        &self,
        spectrum: &[f32],
        config: &DesignConfig,
        params: &DesignParams,
    ) -> Vec<Vertex> {
        let params = match params {
            DesignParams::CircularRadial(p) => p,
            _ => return Vec::new(),
        };

        let bar_count = spectrum.len().min(config.bar_count as usize);
        if bar_count == 0 {
            return Vec::new();
        }

        let ctx = CircularContext::new(config);
        let mut vertices = Vec::with_capacity(bar_count * 6);

        let inner_radius = params.inner_radius * ctx.min_dim * 0.5;
        let radius_range = (params.outer_radius - params.inner_radius) * ctx.min_dim * 0.5;
        let bar_angular_width = params.arc_span / bar_count as f32 * 0.8;
        let half_angle = bar_angular_width * 0.5 * ctx.local_expand;

        for (i, &value) in spectrum.iter().take(bar_count).enumerate() {
            let value = value.clamp(0.0, 1.0);
            let angle = params.start_angle + (i as f32 / bar_count as f32) * params.arc_span + params.rotation;
            let outer_radius = inner_radius + radius_range * value * ctx.beat_scale;

            ctx.push_radial_quad(
                &mut vertices,
                angle,
                half_angle,
                inner_radius,
                outer_radius,
                value,
                i as f32,
            );
        }

        vertices
    }
}

/// Bars arranged around a ring.
pub struct CircularRingDesign;

impl Design for CircularRingDesign {
    fn design_type(&self) -> DesignType {
        DesignType::CircularRing
    }

    fn generate_vertices(
        &self,
        spectrum: &[f32],
        config: &DesignConfig,
        params: &DesignParams,
    ) -> Vec<Vertex> {
        let params = match params {
            DesignParams::CircularRing(p) => p,
            _ => return Vec::new(),
        };

        let bar_count = spectrum.len().min(config.bar_count as usize);
        if bar_count == 0 {
            return Vec::new();
        }

        let ctx = CircularContext::new(config);
        let mut vertices = Vec::with_capacity(bar_count * 6);

        let ring_radius = params.radius * ctx.min_dim * 0.5;
        let max_bar_length = params.bar_length * ctx.min_dim * 0.5;
        let bar_angular_width = 2.0 * PI / bar_count as f32 * 0.7;
        let half_angle = bar_angular_width * 0.5 * ctx.local_expand;

        for (i, &value) in spectrum.iter().take(bar_count).enumerate() {
            let value = value.clamp(0.0, 1.0);
            let angle = (i as f32 / bar_count as f32) * 2.0 * PI + params.rotation;
            let bar_length = max_bar_length * value * ctx.beat_scale;

            let (inner_r, outer_r) = if params.inward {
                (ring_radius - bar_length, ring_radius)
            } else {
                (ring_radius, ring_radius + bar_length)
            };

            ctx.push_radial_quad(
                &mut vertices,
                angle,
                half_angle,
                inner_r,
                outer_r,
                value,
                i as f32,
            );
        }

        vertices
    }
}
