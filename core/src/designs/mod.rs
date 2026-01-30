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

mod bars;
mod circular;
mod frame_corners;
mod frame_perimeter;
mod particles;
mod spectrum_mountain;
mod waveform_line;

pub use bars::BarsDesign;
pub use circular::{CircularRadialDesign, CircularRingDesign};
pub use frame_corners::FrameCornersDesign;
pub use frame_perimeter::FramePerimeterDesign;
pub use particles::{ParticlePattern, ParticlesDesign};
pub use spectrum_mountain::SpectrumMountainDesign;
pub use waveform_line::WaveformLineDesign;

use std::f32::consts::PI;

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
}

impl DesignType {
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
            _ => None,
        }
    }

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
        }
    }

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
        }
    }

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
        ]
    }
}

/// Design-specific parameters.
#[derive(Debug, Clone)]
pub enum DesignParams {
    Bars(BarsParams),
    CircularRadial(CircularRadialParams),
    CircularRing(CircularRingParams),
    FramePerimeter(FramePerimeterParams),
    FrameCorners(FrameCornersParams),
    WaveformLine(WaveformLineParams),
    SpectrumMountain(SpectrumMountainParams),
    Particles(ParticlesParams),
}

impl Default for DesignParams {
    fn default() -> Self {
        Self::Bars(BarsParams::default())
    }
}

/// Parameters for bars design.
#[derive(Debug, Clone)]
pub struct BarsParams {
    pub mirror: bool,
    pub gap_ratio: f32,
    pub vertical: bool,
}

impl Default for BarsParams {
    fn default() -> Self {
        Self {
            mirror: false,
            gap_ratio: 0.1,
            vertical: false,
        }
    }
}

/// Parameters for circular radial design.
#[derive(Debug, Clone)]
pub struct CircularRadialParams {
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub start_angle: f32,
    pub arc_span: f32,
    pub rotation: f32,
}

impl Default for CircularRadialParams {
    fn default() -> Self {
        Self {
            inner_radius: 0.15,
            outer_radius: 0.45,
            start_angle: 0.0,
            arc_span: 2.0 * PI,
            rotation: 0.0,
        }
    }
}

/// Parameters for circular ring design.
#[derive(Debug, Clone)]
pub struct CircularRingParams {
    pub radius: f32,
    pub bar_length: f32,
    pub rotation: f32,
    pub inward: bool,
}

impl Default for CircularRingParams {
    fn default() -> Self {
        Self {
            radius: 0.35,
            bar_length: 0.15,
            rotation: 0.0,
            inward: false,
        }
    }
}

/// Parameters for frame perimeter design.
#[derive(Debug, Clone)]
pub struct FramePerimeterParams {
    /// Distance from screen edge in pixels.
    pub inset: f32,
    /// Thickness of each bar in pixels.
    pub bar_thickness: f32,
    /// Whether bars point inward (true) or outward (false).
    pub inward: bool,
    /// Distribution of bars across edges.
    pub distribution: EdgeDistribution,
}

impl Default for FramePerimeterParams {
    fn default() -> Self {
        Self {
            inset: 20.0,
            bar_thickness: 8.0,
            inward: true,
            distribution: EdgeDistribution::All,
        }
    }
}

/// How bars are distributed across frame edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDistribution {
    /// Distribute bars proportionally across all edges.
    All,
    /// Bars only on top and bottom edges.
    TopBottom,
    /// Bars only on left and right edges.
    LeftRight,
    /// Bars only on top edge.
    TopOnly,
    /// Bars only on bottom edge.
    BottomOnly,
}

/// Parameters for frame corners design.
#[derive(Debug, Clone)]
pub struct FrameCornersParams {
    /// Distance from screen edge in pixels.
    pub inset: f32,
    /// Size of corner area as fraction of min dimension (0.0 - 0.5).
    pub corner_size: f32,
    /// Whether bars point inward (true) or outward (false).
    pub inward: bool,
}

impl Default for FrameCornersParams {
    fn default() -> Self {
        Self {
            inset: 20.0,
            corner_size: 0.25,
            inward: true,
        }
    }
}

/// Parameters for waveform line design.
#[derive(Debug, Clone)]
pub struct WaveformLineParams {
    /// Line thickness in pixels.
    pub line_width: f32,
    /// Smoothing factor (0.0 = none, 1.0 = heavy).
    pub smoothing: f32,
    /// Mirror mode (oscillate above/below center).
    pub mirror: bool,
}

impl Default for WaveformLineParams {
    fn default() -> Self {
        Self {
            line_width: 4.0,
            smoothing: 0.3,
            mirror: true,
        }
    }
}

/// Parameters for spectrum mountain design.
#[derive(Debug, Clone)]
pub struct SpectrumMountainParams {
    /// Baseline position (0.0 = top, 1.0 = bottom).
    pub baseline: f32,
    /// Smoothing factor (0.0 = none, 1.0 = heavy).
    pub smoothing: f32,
    /// Mirror mode (reflect below baseline).
    pub mirror: bool,
}

impl Default for SpectrumMountainParams {
    fn default() -> Self {
        Self {
            baseline: 0.8,
            smoothing: 0.2,
            mirror: false,
        }
    }
}

/// Parameters for particles design.
#[derive(Debug, Clone)]
pub struct ParticlesParams {
    /// Number of particles.
    pub count: u32,
    /// Size range (min, max) in pixels.
    pub size_range: (f32, f32),
    /// Particle distribution pattern.
    pub pattern: ParticlePattern,
}

impl Default for ParticlesParams {
    fn default() -> Self {
        Self {
            count: 200,
            size_range: (4.0, 20.0),
            pattern: ParticlePattern::default(),
        }
    }
}

/// Trait for visualization designs.
pub trait Design: Send + Sync {
    /// Generate vertices for the current frame.
    fn generate_vertices(
        &self,
        spectrum: &[f32],
        config: &DesignConfig,
        params: &DesignParams,
    ) -> Vec<Vertex>;

    /// Design type identifier.
    fn design_type(&self) -> DesignType;
}

/// Create a design instance from type.
pub fn create_design(design_type: DesignType) -> Box<dyn Design> {
    match design_type {
        DesignType::Bars => Box::new(BarsDesign),
        DesignType::CircularRadial => Box::new(CircularRadialDesign),
        DesignType::CircularRing => Box::new(CircularRingDesign),
        DesignType::FramePerimeter => Box::new(FramePerimeterDesign),
        DesignType::FrameCorners => Box::new(FrameCornersDesign),
        DesignType::WaveformLine => Box::new(WaveformLineDesign),
        DesignType::SpectrumMountain => Box::new(SpectrumMountainDesign),
        DesignType::Particles => Box::new(ParticlesDesign),
    }
}

/// Get default params for a design type.
pub fn default_params(design_type: DesignType) -> DesignParams {
    match design_type {
        DesignType::Bars => DesignParams::Bars(BarsParams::default()),
        DesignType::CircularRadial => DesignParams::CircularRadial(CircularRadialParams::default()),
        DesignType::CircularRing => DesignParams::CircularRing(CircularRingParams::default()),
        DesignType::FramePerimeter => DesignParams::FramePerimeter(FramePerimeterParams::default()),
        DesignType::FrameCorners => DesignParams::FrameCorners(FrameCornersParams::default()),
        DesignType::WaveformLine => DesignParams::WaveformLine(WaveformLineParams::default()),
        DesignType::SpectrumMountain => DesignParams::SpectrumMountain(SpectrumMountainParams::default()),
        DesignType::Particles => DesignParams::Particles(ParticlesParams::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Valid inputs
        assert_eq!(DesignType::from_str("bars"), Some(DesignType::Bars));
        assert_eq!(DesignType::from_str("BARS"), Some(DesignType::Bars));
        assert_eq!(DesignType::from_str("radial"), Some(DesignType::CircularRadial));
        assert_eq!(DesignType::from_str("ring"), Some(DesignType::CircularRing));
        // Invalid inputs
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

        // 6 vertices per bar (2 triangles per quad)
        let params = DesignParams::Bars(BarsParams::default());
        let vertices = design.generate_vertices(&spectrum, &config, &params);
        assert_eq!(vertices.len(), 32 * 6);

        // Mirror mode uses same vertex count but changes scaling
        let params_mirror = DesignParams::Bars(BarsParams { mirror: true, ..Default::default() });
        let vertices_mirror = design.generate_vertices(&spectrum, &config, &params_mirror);
        assert_eq!(vertices_mirror.len(), 32 * 6);
    }

    #[test]
    fn test_bars_clamps_spectrum_values() {
        let design = BarsDesign;
        let config = test_config();
        let params = DesignParams::Bars(BarsParams::default());
        let spectrum: Vec<f32> = vec![-0.5, 1.5]; // Out of range values

        let vertices = design.generate_vertices(&spectrum, &config, &params);

        for v in &vertices {
            assert!(v.bar_height >= 0.0 && v.bar_height <= 1.0);
        }
    }

    #[test]
    fn test_bars_vertex_data_correctness() {
        let design = BarsDesign;
        let config = DesignConfig { bar_count: 4, ..test_config() };
        let params = DesignParams::Bars(BarsParams::default());
        let spectrum: Vec<f32> = vec![0.25, 0.5, 0.75, 1.0];

        let vertices = design.generate_vertices(&spectrum, &config, &params);

        // Verify bar indices and heights
        for (bar_idx, chunk) in vertices.chunks(6).enumerate() {
            for v in chunk {
                assert_eq!(v.bar_index as usize, bar_idx);
                assert!((v.bar_height - spectrum[bar_idx]).abs() < 0.001);
            }
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
    fn test_circular_ring_inward_changes_positions() {
        let design = CircularRingDesign;
        let config = test_config();
        let spectrum: Vec<f32> = vec![0.5; 8];

        let params_out = DesignParams::CircularRing(CircularRingParams { inward: false, ..Default::default() });
        let params_in = DesignParams::CircularRing(CircularRingParams { inward: true, ..Default::default() });

        let v_out = design.generate_vertices(&spectrum, &config, &params_out);
        let v_in = design.generate_vertices(&spectrum, &config, &params_in);

        assert_ne!(v_out[0].position, v_in[0].position);
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

    #[test]
    fn test_spectrum_capped_at_bar_count() {
        let design = BarsDesign;
        let config = DesignConfig { bar_count: 8, ..test_config() };
        let params = DesignParams::Bars(BarsParams::default());
        let spectrum: Vec<f32> = vec![0.5; 100]; // Way more than bar_count

        let vertices = design.generate_vertices(&spectrum, &config, &params);
        assert_eq!(vertices.len(), 8 * 6);
    }
}
