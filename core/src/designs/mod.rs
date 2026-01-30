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
            "frame-perimeter" | "frameperimeter" | "perimeter" | "frame" => Some(Self::FramePerimeter),
            "frame-corners" | "framecorners" | "corners" => Some(Self::FrameCorners),
            "waveform-line" | "waveformline" | "line" | "oscilloscope" => Some(Self::WaveformLine),
            "spectrum-mountain" | "spectrummountain" | "mountain" | "area" => Some(Self::SpectrumMountain),
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
        assert_eq!(DesignType::from_str("radial"), Some(DesignType::CircularRadial));
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
            assert!(vertices.is_empty(), "{:?} should produce no vertices for empty spectrum", design_type);
        }
    }
}
