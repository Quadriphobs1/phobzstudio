//! Frame perimeter visualization design.
//!
//! Bars arranged around the frame edge for social media overlays.

use super::{
    Design, DesignConfig, DesignParams, DesignType, EdgeDistribution, FramePerimeterParams, Vertex,
};

/// Rendering context for frame perimeter calculations.
struct FrameContext {
    width: f32,
    height: f32,
    beat_scale: f32,
    local_expand: f32,
}

impl FrameContext {
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
    #[allow(clippy::too_many_arguments)]
    fn push_quad(&self, vertices: &mut Vec<Vertex>, x1: f32, y1: f32, x2: f32, y2: f32, value: f32, index: f32) {
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

/// Edge identifier for bar placement.
#[derive(Clone, Copy)]
enum Edge { Top, Right, Bottom, Left }

impl Edge {
    /// Get position and direction for a bar on this edge.
    fn bar_rect(self, t: f32, ctx: &FrameContext, inset: f32, bar_width: f32, bar_length: f32, inward: bool) -> (f32, f32, f32, f32) {
        let half_w = bar_width * ctx.local_expand * 0.5;
        let length = bar_length * ctx.local_expand;

        match self {
            Edge::Top => {
                let cx = t * ctx.width;
                let base = inset;
                let end = if inward { base + length } else { (base - length).max(0.0) };
                (cx - half_w, base.min(end), cx + half_w, base.max(end))
            }
            Edge::Bottom => {
                let cx = t * ctx.width;
                let base = ctx.height - inset;
                let end = if inward { base - length } else { (base + length).min(ctx.height) };
                (cx - half_w, base.min(end), cx + half_w, base.max(end))
            }
            Edge::Left => {
                let cy = t * ctx.height;
                let base = inset;
                let end = if inward { base + length } else { (base - length).max(0.0) };
                (base.min(end), cy - half_w, base.max(end), cy + half_w)
            }
            Edge::Right => {
                let cy = t * ctx.height;
                let base = ctx.width - inset;
                let end = if inward { base - length } else { (base + length).min(ctx.width) };
                (base.min(end), cy - half_w, base.max(end), cy + half_w)
            }
        }
    }
}

/// Bars arranged around the frame perimeter.
pub struct FramePerimeterDesign;

impl Design for FramePerimeterDesign {
    fn design_type(&self) -> DesignType {
        DesignType::FramePerimeter
    }

    fn generate_vertices(&self, spectrum: &[f32], config: &DesignConfig, params: &DesignParams) -> Vec<Vertex> {
        let params = match params {
            DesignParams::FramePerimeter(p) => p,
            _ => return Vec::new(),
        };

        let bar_count = spectrum.len().min(config.bar_count as usize);
        if bar_count == 0 {
            return Vec::new();
        }

        let ctx = FrameContext::new(config);
        let mut vertices = Vec::with_capacity(bar_count * 6);

        // Calculate uniform bar dimensions
        let perimeter = 2.0 * (ctx.width + ctx.height);
        let bar_slot = perimeter / bar_count as f32;
        let bar_width = bar_slot * 0.85; // 15% gap
        let max_length = (ctx.width.min(ctx.height) * 0.2).max(50.0);

        match params.distribution {
            EdgeDistribution::All => {
                self.generate_perimeter(spectrum, &mut vertices, &ctx, params, bar_width, max_length);
            }
            EdgeDistribution::TopBottom => {
                let half = bar_count / 2;
                self.generate_edge(spectrum, &mut vertices, 0, half, Edge::Top, &ctx, params, bar_width, max_length);
                self.generate_edge(spectrum, &mut vertices, half, bar_count - half, Edge::Bottom, &ctx, params, bar_width, max_length);
            }
            EdgeDistribution::LeftRight => {
                let half = bar_count / 2;
                self.generate_edge(spectrum, &mut vertices, 0, half, Edge::Left, &ctx, params, bar_width, max_length);
                self.generate_edge(spectrum, &mut vertices, half, bar_count - half, Edge::Right, &ctx, params, bar_width, max_length);
            }
            EdgeDistribution::TopOnly => {
                self.generate_edge(spectrum, &mut vertices, 0, bar_count, Edge::Top, &ctx, params, bar_width, max_length);
            }
            EdgeDistribution::BottomOnly => {
                self.generate_edge(spectrum, &mut vertices, 0, bar_count, Edge::Bottom, &ctx, params, bar_width, max_length);
            }
        }

        vertices
    }
}

impl FramePerimeterDesign {
    /// Generate bars around entire perimeter.
    fn generate_perimeter(
        &self,
        spectrum: &[f32],
        vertices: &mut Vec<Vertex>,
        ctx: &FrameContext,
        params: &FramePerimeterParams,
        bar_width: f32,
        max_length: f32,
    ) {
        let perimeter = 2.0 * (ctx.width + ctx.height);
        let bar_count = spectrum.len();
        let spacing = perimeter / bar_count as f32;

        for (i, &value) in spectrum.iter().enumerate() {
            let value = value.clamp(0.0, 1.0);
            let pos = (i as f32 + 0.5) * spacing;

            // Determine edge and position along it
            let (edge, t) = if pos < ctx.width {
                (Edge::Top, pos / ctx.width)
            } else if pos < ctx.width + ctx.height {
                (Edge::Right, (pos - ctx.width) / ctx.height)
            } else if pos < 2.0 * ctx.width + ctx.height {
                (Edge::Bottom, 1.0 - (pos - ctx.width - ctx.height) / ctx.width)
            } else {
                (Edge::Left, 1.0 - (pos - 2.0 * ctx.width - ctx.height) / ctx.height)
            };

            let bar_length = max_length * value * ctx.beat_scale;
            let (x1, y1, x2, y2) = edge.bar_rect(t, ctx, params.inset, bar_width, bar_length, params.inward);
            ctx.push_quad(vertices, x1, y1, x2, y2, value, i as f32);
        }
    }

    /// Generate bars on a single edge.
    #[allow(clippy::too_many_arguments)]
    fn generate_edge(
        &self,
        spectrum: &[f32],
        vertices: &mut Vec<Vertex>,
        start: usize,
        count: usize,
        edge: Edge,
        ctx: &FrameContext,
        params: &FramePerimeterParams,
        bar_width: f32,
        max_length: f32,
    ) {
        for i in 0..count {
            let idx = start + i;
            if idx >= spectrum.len() {
                break;
            }

            let value = spectrum[idx].clamp(0.0, 1.0);
            let t = (i as f32 + 0.5) / count as f32;
            let bar_length = max_length * value * ctx.beat_scale;
            let (x1, y1, x2, y2) = edge.bar_rect(t, ctx, params.inset, bar_width, bar_length, params.inward);
            ctx.push_quad(vertices, x1, y1, x2, y2, value, idx as f32);
        }
    }
}
