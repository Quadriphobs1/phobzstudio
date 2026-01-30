//! Traditional bar waveform design.

use super::{BarsParams, Design, DesignConfig, DesignParams, DesignType, Vertex};

/// Traditional vertical/horizontal bars visualization.
pub struct BarsDesign;

impl Design for BarsDesign {
    fn design_type(&self) -> DesignType {
        DesignType::Bars
    }

    fn generate_vertices(
        &self,
        spectrum: &[f32],
        config: &DesignConfig,
        params: &DesignParams,
    ) -> Vec<Vertex> {
        let params = match params {
            DesignParams::Bars(p) => p,
            _ => &BarsParams::default(),
        };

        let bar_count = spectrum.len().min(config.bar_count as usize);
        let glow_expand = if config.glow { 0.3 } else { 0.0 };
        let beat_scale = 1.0 + config.beat_intensity * 0.15;

        let width = config.width as f32;
        let height = config.height as f32;

        let mut vertices = Vec::with_capacity(bar_count * 6);

        if params.vertical {
            self.generate_vertical_bars(
                spectrum,
                bar_count,
                params,
                width,
                height,
                glow_expand,
                beat_scale,
                &mut vertices,
            );
        } else {
            self.generate_horizontal_bars(
                spectrum,
                bar_count,
                params,
                width,
                height,
                glow_expand,
                beat_scale,
                &mut vertices,
            );
        }

        vertices
    }
}

impl BarsDesign {
    fn generate_horizontal_bars(
        &self,
        spectrum: &[f32],
        bar_count: usize,
        params: &BarsParams,
        width: f32,
        height: f32,
        glow_expand: f32,
        beat_scale: f32,
        vertices: &mut Vec<Vertex>,
    ) {
        let bar_width = width / bar_count as f32;
        let gap = bar_width * params.gap_ratio;
        let actual_bar_width = bar_width - gap;
        let expanded_bar_width = actual_bar_width * (1.0 + glow_expand);

        let height_scale = if params.mirror { 0.4 } else { 0.8 };

        for (i, &bar_height) in spectrum.iter().take(bar_count).enumerate() {
            let bar_height = bar_height.clamp(0.0, 1.0);
            let bar_x = i as f32 * bar_width + gap * 0.5;
            let center_bar_x = bar_x + actual_bar_width * 0.5;

            let scaled_height = bar_height * height * height_scale * beat_scale;
            let half_height = scaled_height * 0.5;
            let expanded_half_height = half_height * (1.0 + glow_expand);
            let center_y = height * 0.5;

            let left = center_bar_x - expanded_bar_width * 0.5;
            let right = center_bar_x + expanded_bar_width * 0.5;
            let top = center_y - expanded_half_height;
            let bottom = center_y + expanded_half_height;

            self.push_quad(
                vertices,
                left,
                right,
                top,
                bottom,
                width,
                height,
                bar_height,
                i as f32,
                glow_expand,
            );
        }
    }

    fn generate_vertical_bars(
        &self,
        spectrum: &[f32],
        bar_count: usize,
        params: &BarsParams,
        width: f32,
        height: f32,
        glow_expand: f32,
        beat_scale: f32,
        vertices: &mut Vec<Vertex>,
    ) {
        let bar_height_px = height / bar_count as f32;
        let gap = bar_height_px * params.gap_ratio;
        let actual_bar_height = bar_height_px - gap;
        let expanded_bar_height = actual_bar_height * (1.0 + glow_expand);

        let width_scale = if params.mirror { 0.4 } else { 0.8 };

        for (i, &bar_height) in spectrum.iter().take(bar_count).enumerate() {
            let bar_height = bar_height.clamp(0.0, 1.0);
            let bar_y = height - (i as f32 + 1.0) * bar_height_px + gap * 0.5;
            let center_bar_y = bar_y + actual_bar_height * 0.5;

            let scaled_width = bar_height * width * width_scale * beat_scale;
            let half_width = scaled_width * 0.5;
            let expanded_half_width = half_width * (1.0 + glow_expand);
            let center_x = width * 0.5;

            let left = center_x - expanded_half_width;
            let right = center_x + expanded_half_width;
            let top = center_bar_y - expanded_bar_height * 0.5;
            let bottom = center_bar_y + expanded_bar_height * 0.5;

            self.push_quad(
                vertices,
                left,
                right,
                top,
                bottom,
                width,
                height,
                bar_height,
                i as f32,
                glow_expand,
            );
        }
    }

    fn push_quad(
        &self,
        vertices: &mut Vec<Vertex>,
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
        width: f32,
        height: f32,
        bar_height: f32,
        bar_index: f32,
        glow_expand: f32,
    ) {
        let to_ndc_x = |x: f32| (x / width) * 2.0 - 1.0;
        let to_ndc_y = |y: f32| 1.0 - (y / height) * 2.0;

        let local_expand = 1.0 + glow_expand;

        let tl = Vertex {
            position: [to_ndc_x(left), to_ndc_y(top)],
            local_pos: [-local_expand, -local_expand],
            bar_height,
            bar_index,
        };
        let tr = Vertex {
            position: [to_ndc_x(right), to_ndc_y(top)],
            local_pos: [local_expand, -local_expand],
            bar_height,
            bar_index,
        };
        let bl = Vertex {
            position: [to_ndc_x(left), to_ndc_y(bottom)],
            local_pos: [-local_expand, local_expand],
            bar_height,
            bar_index,
        };
        let br = Vertex {
            position: [to_ndc_x(right), to_ndc_y(bottom)],
            local_pos: [local_expand, local_expand],
            bar_height,
            bar_index,
        };

        // Two triangles for the quad
        vertices.push(tl);
        vertices.push(bl);
        vertices.push(tr);
        vertices.push(tr);
        vertices.push(bl);
        vertices.push(br);
    }
}
