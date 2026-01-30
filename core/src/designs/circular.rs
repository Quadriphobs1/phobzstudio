//! Circular visualization designs.

use super::{
    CircularRadialParams, CircularRingParams, Design, DesignConfig, DesignParams, DesignType,
    Vertex,
};
use std::f32::consts::PI;

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
            _ => &CircularRadialParams::default(),
        };

        let bar_count = spectrum.len().min(config.bar_count as usize);
        let glow_expand = if config.glow { 0.3 } else { 0.0 };
        let beat_scale = 1.0 + config.beat_intensity * 0.15;

        let width = config.width as f32;
        let height = config.height as f32;
        let center_x = width * 0.5;
        let center_y = height * 0.5;
        let min_dim = width.min(height);

        let mut vertices = Vec::with_capacity(bar_count * 6);

        for (i, &bar_height) in spectrum.iter().take(bar_count).enumerate() {
            let bar_height = bar_height.clamp(0.0, 1.0);

            // Calculate angle for this bar
            let t = i as f32 / bar_count as f32;
            let angle = params.start_angle + t * params.arc_span + params.rotation;

            // Calculate inner and outer radius
            let inner_r = params.inner_radius * min_dim * 0.5;
            let max_bar_length = (params.outer_radius - params.inner_radius) * min_dim * 0.5;
            let bar_length = max_bar_length * bar_height * beat_scale;
            let outer_r = inner_r + bar_length;

            // Bar width (angular)
            let bar_angular_width = params.arc_span / bar_count as f32 * 0.8;
            let half_angle = bar_angular_width * 0.5 * (1.0 + glow_expand);

            // Generate quad vertices
            let cos_l = (angle - half_angle).cos();
            let sin_l = (angle - half_angle).sin();
            let cos_r = (angle + half_angle).cos();
            let sin_r = (angle + half_angle).sin();

            // Expand for glow
            let inner_r_glow = inner_r * (1.0 - glow_expand * 0.5);
            let outer_r_glow = outer_r * (1.0 + glow_expand);

            // Four corners of the bar
            let inner_left = (center_x + cos_l * inner_r_glow, center_y + sin_l * inner_r_glow);
            let inner_right = (center_x + cos_r * inner_r_glow, center_y + sin_r * inner_r_glow);
            let outer_left = (center_x + cos_l * outer_r_glow, center_y + sin_l * outer_r_glow);
            let outer_right = (center_x + cos_r * outer_r_glow, center_y + sin_r * outer_r_glow);

            let to_ndc_x = |x: f32| (x / width) * 2.0 - 1.0;
            let to_ndc_y = |y: f32| 1.0 - (y / height) * 2.0;

            let local_expand = 1.0 + glow_expand;

            let v_il = Vertex {
                position: [to_ndc_x(inner_left.0), to_ndc_y(inner_left.1)],
                local_pos: [-local_expand, -local_expand],
                bar_height,
                bar_index: i as f32,
            };
            let v_ir = Vertex {
                position: [to_ndc_x(inner_right.0), to_ndc_y(inner_right.1)],
                local_pos: [local_expand, -local_expand],
                bar_height,
                bar_index: i as f32,
            };
            let v_ol = Vertex {
                position: [to_ndc_x(outer_left.0), to_ndc_y(outer_left.1)],
                local_pos: [-local_expand, local_expand],
                bar_height,
                bar_index: i as f32,
            };
            let v_or = Vertex {
                position: [to_ndc_x(outer_right.0), to_ndc_y(outer_right.1)],
                local_pos: [local_expand, local_expand],
                bar_height,
                bar_index: i as f32,
            };

            // Two triangles
            vertices.push(v_il);
            vertices.push(v_ol);
            vertices.push(v_ir);
            vertices.push(v_ir);
            vertices.push(v_ol);
            vertices.push(v_or);
        }

        vertices
    }
}

/// Bars arranged around a ring, pointing outward.
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
            _ => &CircularRingParams::default(),
        };

        let bar_count = spectrum.len().min(config.bar_count as usize);
        let glow_expand = if config.glow { 0.3 } else { 0.0 };
        let beat_scale = 1.0 + config.beat_intensity * 0.15;

        let width = config.width as f32;
        let height = config.height as f32;
        let center_x = width * 0.5;
        let center_y = height * 0.5;
        let min_dim = width.min(height);

        let ring_radius = params.radius * min_dim * 0.5;
        let max_bar_length = params.bar_length * min_dim * 0.5;

        let mut vertices = Vec::with_capacity(bar_count * 6);

        for (i, &bar_height) in spectrum.iter().take(bar_count).enumerate() {
            let bar_height = bar_height.clamp(0.0, 1.0);

            // Calculate angle for this bar
            let t = i as f32 / bar_count as f32;
            let angle = t * 2.0 * PI + params.rotation;

            // Bar extends from ring outward (or inward)
            let bar_length = max_bar_length * bar_height * beat_scale;
            let (inner_r, outer_r) = if params.inward {
                (ring_radius - bar_length, ring_radius)
            } else {
                (ring_radius, ring_radius + bar_length)
            };

            // Bar width (angular)
            let bar_angular_width = 2.0 * PI / bar_count as f32 * 0.7;
            let half_angle = bar_angular_width * 0.5 * (1.0 + glow_expand);

            let cos_l = (angle - half_angle).cos();
            let sin_l = (angle - half_angle).sin();
            let cos_r = (angle + half_angle).cos();
            let sin_r = (angle + half_angle).sin();

            // Expand for glow
            let inner_r_glow = inner_r * (1.0 - glow_expand * 0.3);
            let outer_r_glow = outer_r * (1.0 + glow_expand * 0.3);

            let inner_left = (center_x + cos_l * inner_r_glow, center_y + sin_l * inner_r_glow);
            let inner_right = (center_x + cos_r * inner_r_glow, center_y + sin_r * inner_r_glow);
            let outer_left = (center_x + cos_l * outer_r_glow, center_y + sin_l * outer_r_glow);
            let outer_right = (center_x + cos_r * outer_r_glow, center_y + sin_r * outer_r_glow);

            let to_ndc_x = |x: f32| (x / width) * 2.0 - 1.0;
            let to_ndc_y = |y: f32| 1.0 - (y / height) * 2.0;

            let local_expand = 1.0 + glow_expand;

            let v_il = Vertex {
                position: [to_ndc_x(inner_left.0), to_ndc_y(inner_left.1)],
                local_pos: [-local_expand, -local_expand],
                bar_height,
                bar_index: i as f32,
            };
            let v_ir = Vertex {
                position: [to_ndc_x(inner_right.0), to_ndc_y(inner_right.1)],
                local_pos: [local_expand, -local_expand],
                bar_height,
                bar_index: i as f32,
            };
            let v_ol = Vertex {
                position: [to_ndc_x(outer_left.0), to_ndc_y(outer_left.1)],
                local_pos: [-local_expand, local_expand],
                bar_height,
                bar_index: i as f32,
            };
            let v_or = Vertex {
                position: [to_ndc_x(outer_right.0), to_ndc_y(outer_right.1)],
                local_pos: [local_expand, local_expand],
                bar_height,
                bar_index: i as f32,
            };

            vertices.push(v_il);
            vertices.push(v_ol);
            vertices.push(v_ir);
            vertices.push(v_ir);
            vertices.push(v_ol);
            vertices.push(v_or);
        }

        vertices
    }
}
