//! Frame corners visualization design.
//!
//! Bars positioned at corners of a rectangular frame, creating an L-shape
//! at each corner with bars along both the horizontal and vertical edges.

use super::{Design, DesignConfig, DesignParams, DesignType, Vertex};

/// Rendering context for frame corners calculations.
struct CornersContext {
    width: f32,
    height: f32,
    beat_scale: f32,
    local_expand: f32,
}

impl CornersContext {
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

    /// Push a quad defined by bounds.
    fn push_quad(&self, vertices: &mut Vec<Vertex>, x1: f32, y1: f32, x2: f32, y2: f32, value: f32, index: f32) {
        // Ensure coordinates are properly ordered
        let (x1, x2) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
        let (y1, y2) = if y1 < y2 { (y1, y2) } else { (y2, y1) };

        let positions = [
            self.to_ndc(x1, y1), // top-left
            self.to_ndc(x2, y1), // top-right
            self.to_ndc(x1, y2), // bottom-left
            self.to_ndc(x2, y2), // bottom-right
        ];

        let local = self.local_expand;
        let local_positions = [[-local, -local], [local, -local], [-local, local], [local, local]];
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

/// Bars positioned at frame corners.
pub struct FrameCornersDesign;

impl Design for FrameCornersDesign {
    fn design_type(&self) -> DesignType {
        DesignType::FrameCorners
    }

    fn generate_vertices(&self, spectrum: &[f32], config: &DesignConfig, params: &DesignParams) -> Vec<Vertex> {
        let params = match params {
            DesignParams::FrameCorners(p) => p,
            _ => return Vec::new(),
        };

        let bar_count = spectrum.len().min(config.bar_count as usize);
        if bar_count == 0 {
            return Vec::new();
        }

        let ctx = CornersContext::new(config);
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
                        ctx.push_quad(&mut vertices, hx - half_width, hy1, hx + half_width, hy2, value, spectrum_idx as f32);

                        // Vertical bars along left edge, going down from corner
                        let vx = params.inset;
                        let vy = params.inset + offset;
                        let (vx1, vx2) = if params.inward {
                            (vx, vx + bar_length) // grow right (inward)
                        } else {
                            ((vx - bar_length).max(0.0), vx) // grow left (outward)
                        };
                        ctx.push_quad(&mut vertices, vx1, vy - half_width, vx2, vy + half_width, value, spectrum_idx as f32);
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
                        ctx.push_quad(&mut vertices, hx - half_width, hy1, hx + half_width, hy2, value, spectrum_idx as f32);

                        // Vertical bars along right edge, going down from corner
                        let vx = ctx.width - params.inset;
                        let vy = params.inset + offset;
                        let (vx1, vx2) = if params.inward {
                            (vx - bar_length, vx) // grow left (inward)
                        } else {
                            (vx, (vx + bar_length).min(ctx.width)) // grow right (outward)
                        };
                        ctx.push_quad(&mut vertices, vx1, vy - half_width, vx2, vy + half_width, value, spectrum_idx as f32);
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
                        ctx.push_quad(&mut vertices, hx - half_width, hy1, hx + half_width, hy2, value, spectrum_idx as f32);

                        // Vertical bars along right edge, going up from corner
                        let vx = ctx.width - params.inset;
                        let vy = ctx.height - params.inset - offset;
                        let (vx1, vx2) = if params.inward {
                            (vx - bar_length, vx) // grow left (inward)
                        } else {
                            (vx, (vx + bar_length).min(ctx.width)) // grow right (outward)
                        };
                        ctx.push_quad(&mut vertices, vx1, vy - half_width, vx2, vy + half_width, value, spectrum_idx as f32);
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
                        ctx.push_quad(&mut vertices, hx - half_width, hy1, hx + half_width, hy2, value, spectrum_idx as f32);

                        // Vertical bars along left edge, going up from corner
                        let vx = params.inset;
                        let vy = ctx.height - params.inset - offset;
                        let (vx1, vx2) = if params.inward {
                            (vx, vx + bar_length) // grow right (inward)
                        } else {
                            ((vx - bar_length).max(0.0), vx) // grow left (outward)
                        };
                        ctx.push_quad(&mut vertices, vx1, vy - half_width, vx2, vy + half_width, value, spectrum_idx as f32);
                    }
                    _ => unreachable!(),
                }

                spectrum_idx += 1;
            }
        }

        vertices
    }
}
