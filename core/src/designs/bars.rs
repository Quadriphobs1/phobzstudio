//! Traditional bar waveform design.

use super::{
    BarsParams, Design, DesignConfig, DesignParams, DesignType, QuadData, Rect, RenderContext,
    Vertex,
};

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
        if bar_count == 0 {
            return Vec::new();
        }

        let ctx = RenderContext::new(config);
        let mut vertices = Vec::with_capacity(bar_count * 6);

        if params.vertical {
            Self::generate_vertical_bars(spectrum, bar_count, params, &ctx, &mut vertices);
        } else {
            Self::generate_horizontal_bars(spectrum, bar_count, params, &ctx, &mut vertices);
        }

        vertices
    }
}

impl BarsDesign {
    fn generate_horizontal_bars(
        spectrum: &[f32],
        bar_count: usize,
        params: &BarsParams,
        ctx: &RenderContext,
        vertices: &mut Vec<Vertex>,
    ) {
        let bar_width = ctx.width / bar_count as f32;
        let gap = bar_width * params.gap_ratio;
        let actual_bar_width = bar_width - gap;
        let expanded_bar_width = actual_bar_width * ctx.local_expand;
        let height_scale = if params.mirror { 0.4 } else { 0.8 };

        for (i, &value) in spectrum.iter().take(bar_count).enumerate() {
            let value = value.clamp(0.0, 1.0);
            let bar_x = i as f32 * bar_width + gap * 0.5;
            let center_bar_x = bar_x + actual_bar_width * 0.5;

            let scaled_height = value * ctx.height * height_scale * ctx.beat_scale;
            let expanded_half_height = scaled_height * 0.5 * ctx.local_expand;
            let center_y = ctx.height * 0.5;

            ctx.push_quad(
                vertices,
                QuadData {
                    bounds: Rect::new(
                        center_bar_x - expanded_bar_width * 0.5,
                        center_y - expanded_half_height,
                        center_bar_x + expanded_bar_width * 0.5,
                        center_y + expanded_half_height,
                    ),
                    value,
                    index: i as f32,
                },
            );
        }
    }

    fn generate_vertical_bars(
        spectrum: &[f32],
        bar_count: usize,
        params: &BarsParams,
        ctx: &RenderContext,
        vertices: &mut Vec<Vertex>,
    ) {
        let bar_height_px = ctx.height / bar_count as f32;
        let gap = bar_height_px * params.gap_ratio;
        let actual_bar_height = bar_height_px - gap;
        let expanded_bar_height = actual_bar_height * ctx.local_expand;
        let width_scale = if params.mirror { 0.4 } else { 0.8 };

        for (i, &value) in spectrum.iter().take(bar_count).enumerate() {
            let value = value.clamp(0.0, 1.0);
            let bar_y = ctx.height - (i as f32 + 1.0) * bar_height_px + gap * 0.5;
            let center_bar_y = bar_y + actual_bar_height * 0.5;

            let scaled_width = value * ctx.width * width_scale * ctx.beat_scale;
            let expanded_half_width = scaled_width * 0.5 * ctx.local_expand;
            let center_x = ctx.width * 0.5;

            ctx.push_quad(
                vertices,
                QuadData {
                    bounds: Rect::new(
                        center_x - expanded_half_width,
                        center_bar_y - expanded_bar_height * 0.5,
                        center_x + expanded_half_width,
                        center_bar_y + expanded_bar_height * 0.5,
                    ),
                    value,
                    index: i as f32,
                },
            );
        }
    }
}
