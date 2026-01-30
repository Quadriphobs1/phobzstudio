//! Visualization design system.
//!
//! Provides different visual styles for audio visualization:
//! - Bars: Traditional vertical/horizontal bars
//! - CircularRadial: Bars emanating from center
//! - CircularRing: Bars arranged around a ring
//! - FramePerimeter: Bars along screen edges
//! - FrameCorners: Bars positioned at frame corners
//! - WaveformLine: Classic oscilloscope-style line
//! - SpectrumMountain: Filled polygon spectrum
//! - Particles: Beat-reactive particles
//! - Spectrogram: Time-frequency visualization

// Design implementations
mod bars;
mod circular;
mod frame_corners;
mod frame_perimeter;
mod particles;
mod spectrogram;
mod spectrum_mountain;
mod waveform_line;

// Core modules
mod params;
mod registry;

// Re-export design implementations
pub use bars::BarsDesign;
pub use circular::{CircularRadialDesign, CircularRingDesign};
pub use frame_corners::FrameCornersDesign;
pub use frame_perimeter::FramePerimeterDesign;
pub use particles::{ParticlePattern, ParticlesDesign};
pub use spectrogram::{SpectrogramDesign, SpectrogramStyle};
pub use spectrum_mountain::SpectrumMountainDesign;
pub use waveform_line::WaveformLineDesign;

// Re-export params
pub use params::{
    BarsParams, CircularRadialParams, CircularRingParams, DesignParams, EdgeDistribution,
    FrameCornersParams, FramePerimeterParams, ParticlesParams, SpectrogramParams,
    SpectrumMountainParams, WaveformLineParams,
};

// Re-export registry functions
pub use registry::{create_design, default_params};

// ============================================================================
// Core types
// ============================================================================

/// Vertex data for rendering.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub local_pos: [f32; 2],
    pub bar_height: f32,
    pub bar_index: f32,
}

/// Axis-aligned bounding rectangle in pixel coordinates.
#[derive(Copy, Clone, Debug, Default)]
pub struct Rect {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl Rect {
    /// Create a new rectangle from corner coordinates.
    #[inline]
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// Ensure coordinates are properly ordered (min to max).
    #[inline]
    pub fn normalized(self) -> Self {
        let (x1, x2) = if self.x1 < self.x2 {
            (self.x1, self.x2)
        } else {
            (self.x2, self.x1)
        };
        let (y1, y2) = if self.y1 < self.y2 {
            (self.y1, self.y2)
        } else {
            (self.y2, self.y1)
        };
        Self { x1, y1, x2, y2 }
    }
}

/// Data for a single quad (bar/cell) to be rendered.
#[derive(Copy, Clone, Debug, Default)]
pub struct QuadData {
    pub bounds: Rect,
    pub value: f32,
    pub index: f32,
}

/// Shared rendering context for all designs.
///
/// Provides common utilities for coordinate conversion and vertex generation.
#[derive(Copy, Clone, Debug)]
pub struct RenderContext {
    pub width: f32,
    pub height: f32,
    pub beat_scale: f32,
    pub local_expand: f32,
}

impl RenderContext {
    /// Create a new render context from design config.
    pub fn new(config: &DesignConfig) -> Self {
        let glow_expand = if config.glow { 0.3 } else { 0.0 };
        Self {
            width: config.width as f32,
            height: config.height as f32,
            beat_scale: 1.0 + config.beat_intensity * 0.15,
            local_expand: 1.0 + glow_expand,
        }
    }

    /// Convert pixel coordinates to normalized device coordinates.
    #[inline]
    pub fn to_ndc(&self, x: f32, y: f32) -> [f32; 2] {
        [(x / self.width) * 2.0 - 1.0, 1.0 - (y / self.height) * 2.0]
    }

    /// Push a quad to the vertex buffer.
    pub fn push_quad(&self, vertices: &mut Vec<Vertex>, quad: QuadData) {
        let bounds = quad.bounds.normalized();
        let positions = [
            self.to_ndc(bounds.x1, bounds.y1),
            self.to_ndc(bounds.x2, bounds.y1),
            self.to_ndc(bounds.x1, bounds.y2),
            self.to_ndc(bounds.x2, bounds.y2),
        ];

        let local = self.local_expand;
        let local_positions = [
            [-local, -local],
            [local, -local],
            [-local, local],
            [local, local],
        ];

        const INDICES: [usize; 6] = [0, 2, 1, 1, 2, 3];
        for &idx in &INDICES {
            vertices.push(Vertex {
                position: positions[idx],
                local_pos: local_positions[idx],
                bar_height: quad.value,
                bar_index: quad.index,
            });
        }
    }
}

/// Common configuration for all designs.
#[derive(Debug, Clone)]
pub struct DesignConfig {
    pub width: u32,
    pub height: u32,
    pub color: [f32; 3],
    pub background: [f32; 3],
    pub bar_count: u32,
    pub glow: bool,
    pub beat_intensity: f32,
}

impl Default for DesignConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            color: [0.0, 1.0, 0.53],
            background: [0.0, 0.0, 0.0],
            bar_count: 64,
            glow: true,
            beat_intensity: 0.0,
        }
    }
}

// ============================================================================
// Design type enumeration
// ============================================================================

/// Available design types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DesignType {
    Bars,
    CircularRadial,
    CircularRing,
    FramePerimeter,
    FrameCorners,
    WaveformLine,
    SpectrumMountain,
    Particles,
    Spectrogram,
}

impl DesignType {
    /// Parse a design type from a string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bars" => Some(Self::Bars),
            "circular-radial" | "circularradial" | "radial" => Some(Self::CircularRadial),
            "circular-ring" | "circularring" | "ring" => Some(Self::CircularRing),
            "frame-perimeter" | "frameperimeter" | "perimeter" | "frame" => {
                Some(Self::FramePerimeter)
            }
            "frame-corners" | "framecorners" | "corners" => Some(Self::FrameCorners),
            "waveform-line" | "waveformline" | "line" | "oscilloscope" => Some(Self::WaveformLine),
            "spectrum-mountain" | "spectrummountain" | "mountain" | "area" => {
                Some(Self::SpectrumMountain)
            }
            "particles" | "particle" => Some(Self::Particles),
            "spectrogram" | "spectro" | "frequency" => Some(Self::Spectrogram),
            _ => None,
        }
    }

    /// Get the canonical name for this design type.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Bars => "bars",
            Self::CircularRadial => "circular-radial",
            Self::CircularRing => "circular-ring",
            Self::FramePerimeter => "frame-perimeter",
            Self::FrameCorners => "frame-corners",
            Self::WaveformLine => "waveform-line",
            Self::SpectrumMountain => "spectrum-mountain",
            Self::Particles => "particles",
            Self::Spectrogram => "spectrogram",
        }
    }

    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Bars => "Traditional vertical/horizontal bars",
            Self::CircularRadial => "Bars emanating outward from center",
            Self::CircularRing => "Bars arranged around a ring",
            Self::FramePerimeter => "Bars along screen edges (overlay)",
            Self::FrameCorners => "Bars at frame corners",
            Self::WaveformLine => "Classic oscilloscope-style line",
            Self::SpectrumMountain => "Filled polygon spectrum",
            Self::Particles => "Beat-reactive particles",
            Self::Spectrogram => "Frequency bands (spectrogram style)",
        }
    }

    /// Get all available design types.
    pub fn all() -> &'static [Self] {
        &[
            Self::Bars,
            Self::CircularRadial,
            Self::CircularRing,
            Self::FramePerimeter,
            Self::FrameCorners,
            Self::WaveformLine,
            Self::SpectrumMountain,
            Self::Particles,
            Self::Spectrogram,
        ]
    }
}

// ============================================================================
// Design trait
// ============================================================================

/// Trait for visualization designs.
///
/// Implement this trait to create custom visualization styles.
/// Each design generates vertices based on audio spectrum data.
pub trait Design: Send + Sync {
    /// Generate vertices for the current frame.
    ///
    /// # Arguments
    /// * `spectrum` - Frequency spectrum values (0.0-1.0)
    /// * `config` - Common configuration (dimensions, colors, etc.)
    /// * `params` - Design-specific parameters
    ///
    /// # Returns
    /// Vector of vertices to render. Each quad uses 6 vertices (2 triangles).
    fn generate_vertices(
        &self,
        spectrum: &[f32],
        config: &DesignConfig,
        params: &DesignParams,
    ) -> Vec<Vertex>;

    /// Design type identifier.
    fn design_type(&self) -> DesignType;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn test_config() -> DesignConfig {
        DesignConfig {
            width: 640,
            height: 480,
            color: [0.0, 1.0, 0.5],
            background: [0.0, 0.0, 0.0],
            bar_count: 32,
            glow: true,
            beat_intensity: 0.0,
        }
    }

    #[test]
    fn test_design_type_from_str_parsing() {
        assert_eq!(DesignType::from_str("bars"), Some(DesignType::Bars));
        assert_eq!(DesignType::from_str("BARS"), Some(DesignType::Bars));
        assert_eq!(
            DesignType::from_str("radial"),
            Some(DesignType::CircularRadial)
        );
        assert_eq!(DesignType::from_str("ring"), Some(DesignType::CircularRing));
        assert_eq!(DesignType::from_str("invalid"), None);
    }

    #[test]
    fn test_create_design_returns_correct_type() {
        for design_type in DesignType::all() {
            let design = create_design(*design_type);
            assert_eq!(design.design_type(), *design_type);
        }
    }

    #[test]
    fn test_bars_vertex_count() {
        let design = BarsDesign;
        let config = test_config();
        let spectrum: Vec<f32> = vec![0.5; 32];

        let params = DesignParams::Bars(BarsParams::default());
        let vertices = design.generate_vertices(&spectrum, &config, &params);
        assert_eq!(vertices.len(), 32 * 6);
    }

    #[test]
    fn test_bars_clamps_spectrum_values() {
        let design = BarsDesign;
        let config = test_config();
        let params = DesignParams::Bars(BarsParams::default());
        let spectrum: Vec<f32> = vec![-0.5, 1.5];

        let vertices = design.generate_vertices(&spectrum, &config, &params);

        for v in &vertices {
            assert!(v.bar_height >= 0.0 && v.bar_height <= 1.0);
        }
    }

    #[test]
    fn test_circular_radial_rotation_changes_positions() {
        let design = CircularRadialDesign;
        let config = test_config();
        let spectrum: Vec<f32> = vec![0.5; 8];

        let params_no_rot = DesignParams::CircularRadial(CircularRadialParams::default());
        let params_rotated = DesignParams::CircularRadial(CircularRadialParams {
            rotation: PI / 2.0,
            ..Default::default()
        });

        let v1 = design.generate_vertices(&spectrum, &config, &params_no_rot);
        let v2 = design.generate_vertices(&spectrum, &config, &params_rotated);

        assert_eq!(v1.len(), v2.len());
        assert_ne!(v1[0].position, v2[0].position);
    }

    #[test]
    fn test_empty_spectrum_produces_no_vertices() {
        let config = test_config();
        let spectrum: Vec<f32> = vec![];

        for design_type in DesignType::all() {
            let design = create_design(*design_type);
            let params = default_params(*design_type);
            let vertices = design.generate_vertices(&spectrum, &config, &params);
            assert!(
                vertices.is_empty(),
                "{:?} should produce no vertices for empty spectrum",
                design_type
            );
        }
    }
}
