//! Frame perimeter visualization design.
//!
//! Bars arranged around the entire frame edge, ideal for social media overlays.

use super::{
    Design, DesignConfig, DesignParams, DesignType, EdgeDistribution, FramePerimeterParams, Vertex,
};

/// Bars arranged around the frame perimeter.
pub struct FramePerimeterDesign;

impl Design for FramePerimeterDesign {
    fn design_type(&self) -> DesignType {
        DesignType::FramePerimeter
    }

    fn generate_vertices(
        &self,
        spectrum: &[f32],
        config: &DesignConfig,
        params: &DesignParams,
    ) -> Vec<Vertex> {
        let params = match params {
            DesignParams::FramePerimeter(p) => p,
            _ => &FramePerimeterParams::default(),
        };

        let bar_count = spectrum.len().min(config.bar_count as usize);
        if bar_count == 0 {
            return Vec::new();
        }

        let glow_expand = if config.glow { 0.3 } else { 0.0 };
        let beat_scale = 1.0 + config.beat_intensity * 0.15;

        let width = config.width as f32;
        let height = config.height as f32;

        // Calculate perimeter and uniform bar spacing
        let perimeter = 2.0 * (width + height);

        // Each bar takes the same amount of space along the perimeter
        let bar_slot = perimeter / bar_count as f32;
        let gap_ratio = 0.15; // 15% gap between bars
        let uniform_bar_width = bar_slot * (1.0 - gap_ratio);

        // Maximum bar length (how far bars extend inward/outward)
        let max_bar_length = (width.min(height) * 0.2).max(50.0);

        let mut vertices = Vec::with_capacity(bar_count * 6);

        // Distribute bars around the perimeter based on distribution mode
        match params.distribution {
            EdgeDistribution::All => {
                self.generate_perimeter_bars(
                    spectrum,
                    &mut vertices,
                    bar_count,
                    width,
                    height,
                    params.inset,
                    uniform_bar_width,
                    max_bar_length,
                    params.inward,
                    glow_expand,
                    beat_scale,
                );
            }
            EdgeDistribution::TopBottom => {
                let half = bar_count / 2;
                self.generate_edge_only(
                    spectrum, &mut vertices, 0, half,
                    Edge::Top, width, height, params.inset,
                    uniform_bar_width, max_bar_length, params.inward, glow_expand, beat_scale,
                );
                self.generate_edge_only(
                    spectrum, &mut vertices, half, bar_count - half,
                    Edge::Bottom, width, height, params.inset,
                    uniform_bar_width, max_bar_length, params.inward, glow_expand, beat_scale,
                );
            }
            EdgeDistribution::LeftRight => {
                let half = bar_count / 2;
                self.generate_edge_only(
                    spectrum, &mut vertices, 0, half,
                    Edge::Left, width, height, params.inset,
                    uniform_bar_width, max_bar_length, params.inward, glow_expand, beat_scale,
                );
                self.generate_edge_only(
                    spectrum, &mut vertices, half, bar_count - half,
                    Edge::Right, width, height, params.inset,
                    uniform_bar_width, max_bar_length, params.inward, glow_expand, beat_scale,
                );
            }
            EdgeDistribution::TopOnly => {
                self.generate_edge_only(
                    spectrum, &mut vertices, 0, bar_count,
                    Edge::Top, width, height, params.inset,
                    uniform_bar_width, max_bar_length, params.inward, glow_expand, beat_scale,
                );
            }
            EdgeDistribution::BottomOnly => {
                self.generate_edge_only(
                    spectrum, &mut vertices, 0, bar_count,
                    Edge::Bottom, width, height, params.inset,
                    uniform_bar_width, max_bar_length, params.inward, glow_expand, beat_scale,
                );
            }
        }

        vertices
    }
}

#[derive(Clone, Copy)]
enum Edge {
    Top,
    Right,
    Bottom,
    Left,
}

impl FramePerimeterDesign {
    /// Generate bars distributed around the entire perimeter
    #[allow(clippy::too_many_arguments)]
    fn generate_perimeter_bars(
        &self,
        spectrum: &[f32],
        vertices: &mut Vec<Vertex>,
        bar_count: usize,
        width: f32,
        height: f32,
        inset: f32,
        bar_width: f32,
        max_length: f32,
        inward: bool,
        glow_expand: f32,
        beat_scale: f32,
    ) {
        let perimeter = 2.0 * (width + height);
        let bar_spacing = perimeter / bar_count as f32;

        for i in 0..bar_count {
            if i >= spectrum.len() {
                break;
            }

            let value = spectrum[i].clamp(0.0, 1.0);
            let bar_length = max_length * value * beat_scale;

            // Position along perimeter (0 to perimeter)
            let pos = (i as f32 + 0.5) * bar_spacing;

            // Determine which edge and position on that edge
            let (edge, edge_pos) = self.perimeter_to_edge(pos, width, height);

            self.generate_single_bar(
                vertices, edge, edge_pos, width, height, inset,
                bar_width, bar_length, inward, glow_expand, value, i as f32,
            );
        }
    }

    /// Convert perimeter position to edge and position on that edge
    fn perimeter_to_edge(&self, pos: f32, width: f32, height: f32) -> (Edge, f32) {
        // Perimeter order: Top (0 to width), Right (width to width+height),
        // Bottom (width+height to 2*width+height), Left (2*width+height to perimeter)
        if pos < width {
            (Edge::Top, pos)
        } else if pos < width + height {
            (Edge::Right, pos - width)
        } else if pos < 2.0 * width + height {
            (Edge::Bottom, width - (pos - width - height)) // Reverse direction
        } else {
            (Edge::Left, height - (pos - 2.0 * width - height)) // Reverse direction
        }
    }

    /// Generate bars on a single edge only
    #[allow(clippy::too_many_arguments)]
    fn generate_edge_only(
        &self,
        spectrum: &[f32],
        vertices: &mut Vec<Vertex>,
        spectrum_start: usize,
        count: usize,
        edge: Edge,
        width: f32,
        height: f32,
        inset: f32,
        bar_width: f32,
        max_length: f32,
        inward: bool,
        glow_expand: f32,
        beat_scale: f32,
    ) {
        let edge_length = match edge {
            Edge::Top | Edge::Bottom => width,
            Edge::Left | Edge::Right => height,
        };

        let bar_spacing = edge_length / count as f32;

        for i in 0..count {
            let spectrum_idx = spectrum_start + i;
            if spectrum_idx >= spectrum.len() {
                break;
            }

            let value = spectrum[spectrum_idx].clamp(0.0, 1.0);
            let bar_length = max_length * value * beat_scale;
            let edge_pos = (i as f32 + 0.5) * bar_spacing;

            self.generate_single_bar(
                vertices, edge, edge_pos, width, height, inset,
                bar_width, bar_length, inward, glow_expand, value, spectrum_idx as f32,
            );
        }
    }

    /// Generate a single bar at the specified position
    #[allow(clippy::too_many_arguments)]
    fn generate_single_bar(
        &self,
        vertices: &mut Vec<Vertex>,
        edge: Edge,
        edge_pos: f32,
        width: f32,
        height: f32,
        inset: f32,
        bar_width: f32,
        bar_length: f32,
        inward: bool,
        glow_expand: f32,
        value: f32,
        bar_index: f32,
    ) {
        let expanded_bar_width = bar_width * (1.0 + glow_expand);
        let expanded_bar_length = bar_length * (1.0 + glow_expand);
        let half_width = expanded_bar_width / 2.0;

        // Calculate bar rectangle based on edge
        let (x1, y1, x2, y2) = match edge {
            Edge::Top => {
                let cx = edge_pos.clamp(half_width, width - half_width);
                let base_y = inset;
                let end_y = if inward {
                    base_y + expanded_bar_length
                } else {
                    0.0_f32.max(base_y - expanded_bar_length)
                };
                (cx - half_width, base_y.min(end_y), cx + half_width, base_y.max(end_y))
            }
            Edge::Bottom => {
                let cx = edge_pos.clamp(half_width, width - half_width);
                let base_y = height - inset;
                let end_y = if inward {
                    base_y - expanded_bar_length
                } else {
                    height.min(base_y + expanded_bar_length)
                };
                (cx - half_width, base_y.min(end_y), cx + half_width, base_y.max(end_y))
            }
            Edge::Left => {
                let cy = edge_pos.clamp(half_width, height - half_width);
                let base_x = inset;
                let end_x = if inward {
                    base_x + expanded_bar_length
                } else {
                    0.0_f32.max(base_x - expanded_bar_length)
                };
                (base_x.min(end_x), cy - half_width, base_x.max(end_x), cy + half_width)
            }
            Edge::Right => {
                let cy = edge_pos.clamp(half_width, height - half_width);
                let base_x = width - inset;
                let end_x = if inward {
                    base_x - expanded_bar_length
                } else {
                    width.min(base_x + expanded_bar_length)
                };
                (base_x.min(end_x), cy - half_width, base_x.max(end_x), cy + half_width)
            }
        };

        self.push_quad(vertices, x1, x2, y1, y2, width, height, value, bar_index, glow_expand);
    }

    #[allow(clippy::too_many_arguments)]
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
        // Convert pixel coordinates to NDC (-1 to 1)
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
