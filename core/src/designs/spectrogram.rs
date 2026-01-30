//! Spectrogram visualization design.
//!
//! Displays a time-frequency representation where:
//! - X-axis represents time (scrolling left, newest on right)
//! - Y-axis represents frequency (low at bottom, high at top)
//! - Color intensity represents magnitude at that time-frequency point

use std::sync::RwLock;

use super::{
    Design, DesignConfig, DesignParams, DesignType, QuadData, Rect, RenderContext, Vertex,
};

/// Spectrogram-style frequency visualization with time history.
///
/// This design maintains an internal history buffer that accumulates
/// spectrum data over time, creating a scrolling time-frequency display.
pub struct SpectrogramDesign {
    /// Rolling history buffer: Vec of spectrum frames.
    /// Oldest frames at index 0, newest at the end.
    history: RwLock<Vec<Vec<f32>>>,
}

impl Default for SpectrogramDesign {
    fn default() -> Self {
        Self {
            history: RwLock::new(Vec::new()),
        }
    }
}

impl SpectrogramDesign {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear the history buffer.
    pub fn clear_history(&self) {
        self.history.write().unwrap().clear();
    }
}

impl Design for SpectrogramDesign {
    fn design_type(&self) -> DesignType {
        DesignType::Spectrogram
    }

    fn generate_vertices(
        &self,
        spectrum: &[f32],
        config: &DesignConfig,
        params: &DesignParams,
    ) -> Vec<Vertex> {
        let params = match params {
            DesignParams::Spectrogram(p) => p,
            _ => return Vec::new(),
        };

        let freq_bins = spectrum.len().min(config.bar_count as usize);
        if freq_bins == 0 {
            return Vec::new();
        }

        let ctx = RenderContext::new(config);

        // Update history with new spectrum
        {
            let mut history = self.history.write().unwrap();

            // Add new spectrum frame (clamped to 0-1)
            let new_frame: Vec<f32> = spectrum
                .iter()
                .take(freq_bins)
                .map(|&v| v.clamp(0.0, 1.0))
                .collect();
            history.push(new_frame);

            // Limit history size based on time_window
            while history.len() > params.time_window {
                history.remove(0);
            }
        }

        let history = self.history.read().unwrap();
        let time_frames = history.len();

        if time_frames == 0 {
            return Vec::new();
        }

        // Calculate cell dimensions
        let margin_x = ctx.width * params.margin;
        let margin_y = ctx.height * params.margin;
        let available_width = ctx.width - 2.0 * margin_x;
        let available_height = ctx.height - 2.0 * margin_y;

        let cell_width = available_width / params.time_window as f32;
        let cell_height = available_height / freq_bins as f32;

        // Gap between cells (small for spectrogram look)
        let gap_x = cell_width * params.gap_ratio * 0.5;
        let gap_y = cell_height * params.gap_ratio * 0.5;

        // Pre-allocate vertices: time_frames * freq_bins * 6 vertices per cell
        let mut vertices = Vec::with_capacity(time_frames * freq_bins * 6);

        // Render each cell in the spectrogram grid
        // X = time (oldest on left, newest on right)
        // Y = frequency (low at bottom, high at top)
        for (time_idx, frame) in history.iter().enumerate() {
            // Position in the time window (oldest frames start from left)
            // When history is not full, frames should still appear on the right
            let time_offset = params.time_window - time_frames + time_idx;
            let x_start = margin_x + time_offset as f32 * cell_width + gap_x;
            let x_end = x_start + cell_width - 2.0 * gap_x;

            for (freq_idx, &value) in frame.iter().enumerate() {
                // Low frequencies at bottom (high Y in screen coords)
                // So we reverse: freq_idx 0 (lowest) should be at bottom
                let reversed_freq = freq_bins - 1 - freq_idx;
                let y_start = margin_y + reversed_freq as f32 * cell_height + gap_y;
                let y_end = y_start + cell_height - 2.0 * gap_y;

                // Scale value by beat intensity
                let scaled_value = (value * ctx.beat_scale).clamp(0.0, 1.0);

                // Use freq_idx as bar_index so the shader can color by frequency
                ctx.push_quad(
                    &mut vertices,
                    QuadData {
                        bounds: Rect::new(x_start, y_start, x_end, y_end),
                        value: scaled_value,
                        index: freq_idx as f32,
                    },
                );
            }
        }

        vertices
    }
}

/// Visual style for spectrogram display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpectrogramStyle {
    /// Standard scrolling spectrogram (time flows left to right).
    #[default]
    Scrolling,
    /// Waterfall style (time flows top to bottom).
    Waterfall,
}
