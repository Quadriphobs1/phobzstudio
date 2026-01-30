//! Frame corners visualization design.
//!
//! Bars positioned at corners of a rectangular frame, creating an L-shape
//! at each corner with bars along both the horizontal and vertical edges.

use super::{
    Design, DesignConfig, DesignParams, DesignType, QuadData, Rect, RenderContext, Vertex,
};

/// Bars positioned at frame corners.
pub struct FrameCornersDesign;

impl Design for FrameCornersDesign {
    fn design_type(&self) -> DesignType {
        DesignType::FrameCorners
    }

    fn generate_vertices(
        &self,
        spectrum: &[f32],
        config: &DesignConfig,
        params: &DesignParams,
    ) -> Vec<Vertex> {
        let params = match params {
            DesignParams::FrameCorners(p) => p,
            _ => return Vec::new(),
        };

        let bar_count = spectrum.len().min(config.bar_count as usize);
        if bar_count == 0 {
            return Vec::new();
        }

        let ctx = RenderContext::new(config);
        // Each bar creates 2 quads (horizontal + vertical), each quad = 6 vertices
        let mut vertices = Vec::with_capacity(bar_count * 12);

        // Calculate corner size in pixels
        let min_dim = ctx.width.min(ctx.height);
        let corner_extent = min_dim * params.corner_size;
        let max_bar_length = corner_extent * 0.6;

        // Distribute bars across 4 corners
        let bars_per_corner = bar_count / 4;
        let extra_bars = bar_count % 4;

        let mut spectrum_idx = 0;

        // Process each corner
        for corner_idx in 0..4 {
            let corner_bar_count = bars_per_corner + if corner_idx < extra_bars { 1 } else { 0 };
            if corner_bar_count == 0 {
                continue;
            }

            let bar_spacing = corner_extent / (corner_bar_count as f32 + 1.0);
            let bar_width = bar_spacing * 0.6 * ctx.local_expand;

            for i in 0..corner_bar_count {
                if spectrum_idx >= spectrum.len() {
                    break;
                }

                let value = spectrum[spectrum_idx].clamp(0.0, 1.0);
                let bar_length = max_bar_length * value * ctx.beat_scale * ctx.local_expand;
                let offset = bar_spacing * (i as f32 + 1.0);
                let half_width = bar_width * 0.5;

                match corner_idx {
                    0 => {
                        // Top-Left corner
                        // Horizontal bars along top edge, going right from corner
                        let hx = params.inset + offset;
                        let hy = params.inset;
                        let (hy1, hy2) = if params.inward {
                            (hy, hy + bar_length) // grow down (inward)
                        } else {
                            ((hy - bar_length).max(0.0), hy) // grow up (outward)
                        };
                        ctx.push_quad(
                            &mut vertices,
                            QuadData {
                                bounds: Rect::new(hx - half_width, hy1, hx + half_width, hy2),
                                value,
                                index: spectrum_idx as f32,
                            },
                        );

                        // Vertical bars along left edge, going down from corner
                        let vx = params.inset;
                        let vy = params.inset + offset;
                        let (vx1, vx2) = if params.inward {
                            (vx, vx + bar_length) // grow right (inward)
                        } else {
                            ((vx - bar_length).max(0.0), vx) // grow left (outward)
                        };
                        ctx.push_quad(
                            &mut vertices,
                            QuadData {
                                bounds: Rect::new(vx1, vy - half_width, vx2, vy + half_width),
                                value,
                                index: spectrum_idx as f32,
                            },
                        );
                    }
                    1 => {
                        // Top-Right corner
                        // Horizontal bars along top edge, going left from corner
                        let hx = ctx.width - params.inset - offset;
                        let hy = params.inset;
                        let (hy1, hy2) = if params.inward {
                            (hy, hy + bar_length) // grow down (inward)
                        } else {
                            ((hy - bar_length).max(0.0), hy) // grow up (outward)
                        };
                        ctx.push_quad(
                            &mut vertices,
                            QuadData {
                                bounds: Rect::new(hx - half_width, hy1, hx + half_width, hy2),
                                value,
                                index: spectrum_idx as f32,
                            },
                        );

                        // Vertical bars along right edge, going down from corner
                        let vx = ctx.width - params.inset;
                        let vy = params.inset + offset;
                        let (vx1, vx2) = if params.inward {
                            (vx - bar_length, vx) // grow left (inward)
                        } else {
                            (vx, (vx + bar_length).min(ctx.width)) // grow right (outward)
                        };
                        ctx.push_quad(
                            &mut vertices,
                            QuadData {
                                bounds: Rect::new(vx1, vy - half_width, vx2, vy + half_width),
                                value,
                                index: spectrum_idx as f32,
                            },
                        );
                    }
                    2 => {
                        // Bottom-Right corner
                        // Horizontal bars along bottom edge, going left from corner
                        let hx = ctx.width - params.inset - offset;
                        let hy = ctx.height - params.inset;
                        let (hy1, hy2) = if params.inward {
                            (hy - bar_length, hy) // grow up (inward)
                        } else {
                            (hy, (hy + bar_length).min(ctx.height)) // grow down (outward)
                        };
                        ctx.push_quad(
                            &mut vertices,
                            QuadData {
                                bounds: Rect::new(hx - half_width, hy1, hx + half_width, hy2),
                                value,
                                index: spectrum_idx as f32,
                            },
                        );

                        // Vertical bars along right edge, going up from corner
                        let vx = ctx.width - params.inset;
                        let vy = ctx.height - params.inset - offset;
                        let (vx1, vx2) = if params.inward {
                            (vx - bar_length, vx) // grow left (inward)
                        } else {
                            (vx, (vx + bar_length).min(ctx.width)) // grow right (outward)
                        };
                        ctx.push_quad(
                            &mut vertices,
                            QuadData {
                                bounds: Rect::new(vx1, vy - half_width, vx2, vy + half_width),
                                value,
                                index: spectrum_idx as f32,
                            },
                        );
                    }
                    3 => {
                        // Bottom-Left corner
                        // Horizontal bars along bottom edge, going right from corner
                        let hx = params.inset + offset;
                        let hy = ctx.height - params.inset;
                        let (hy1, hy2) = if params.inward {
                            (hy - bar_length, hy) // grow up (inward)
                        } else {
                            (hy, (hy + bar_length).min(ctx.height)) // grow down (outward)
                        };
                        ctx.push_quad(
                            &mut vertices,
                            QuadData {
                                bounds: Rect::new(hx - half_width, hy1, hx + half_width, hy2),
                                value,
                                index: spectrum_idx as f32,
                            },
                        );

                        // Vertical bars along left edge, going up from corner
                        let vx = params.inset;
                        let vy = ctx.height - params.inset - offset;
                        let (vx1, vx2) = if params.inward {
                            (vx, vx + bar_length) // grow right (inward)
                        } else {
                            ((vx - bar_length).max(0.0), vx) // grow left (outward)
                        };
                        ctx.push_quad(
                            &mut vertices,
                            QuadData {
                                bounds: Rect::new(vx1, vy - half_width, vx2, vy + half_width),
                                value,
                                index: spectrum_idx as f32,
                            },
                        );
                    }
                    _ => unreachable!(),
                }

                spectrum_idx += 1;
            }
        }

        vertices
    }
}
