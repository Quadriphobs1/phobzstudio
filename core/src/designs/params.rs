//! Design parameter types.
//!
//! Contains all configuration parameters for each visualization design.

use std::f32::consts::PI;

use super::{ParticlePattern, SpectrogramStyle};

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
    Spectrogram(SpectrogramParams),
}

impl Default for DesignParams {
    fn default() -> Self {
        Self::Bars(BarsParams::default())
    }
}

// ============================================================================
// Bar-based designs
// ============================================================================

/// Parameters for bars design.
#[derive(Debug, Clone)]
pub struct BarsParams {
    /// Mirror mode - bars grow from center in both directions.
    pub mirror: bool,
    /// Gap between bars as fraction of bar width (0.0 - 1.0).
    pub gap_ratio: f32,
    /// Vertical layout (bars arranged vertically instead of horizontally).
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

// ============================================================================
// Circular designs
// ============================================================================

/// Parameters for circular radial design.
#[derive(Debug, Clone)]
pub struct CircularRadialParams {
    /// Inner radius as fraction of min dimension (0.0 - 1.0).
    pub inner_radius: f32,
    /// Outer radius as fraction of min dimension (0.0 - 1.0).
    pub outer_radius: f32,
    /// Starting angle in radians.
    pub start_angle: f32,
    /// Arc span in radians (2*PI = full circle).
    pub arc_span: f32,
    /// Rotation offset in radians.
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
    /// Ring radius as fraction of min dimension (0.0 - 1.0).
    pub radius: f32,
    /// Bar length as fraction of min dimension (0.0 - 1.0).
    pub bar_length: f32,
    /// Rotation offset in radians.
    pub rotation: f32,
    /// Bars point inward toward center instead of outward.
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

// ============================================================================
// Frame designs
// ============================================================================

/// How bars are distributed across frame edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EdgeDistribution {
    /// Distribute bars proportionally across all edges.
    #[default]
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

// ============================================================================
// Waveform designs
// ============================================================================

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

// ============================================================================
// Particle designs
// ============================================================================

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

// ============================================================================
// Spectrogram designs
// ============================================================================

/// Parameters for spectrogram design.
#[derive(Debug, Clone)]
pub struct SpectrogramParams {
    /// Margin from edges as fraction of screen (0.0 - 0.5).
    pub margin: f32,
    /// Gap between cells as fraction of cell size (0.0 - 1.0).
    /// Use 0.0 for a continuous look like traditional spectrograms.
    pub gap_ratio: f32,
    /// Number of time frames to display (history length).
    /// At 30fps, 150 frames = 5 seconds of history.
    pub time_window: usize,
    /// Visual style for the spectrogram.
    pub style: SpectrogramStyle,
}

impl Default for SpectrogramParams {
    fn default() -> Self {
        Self {
            margin: 0.02,
            gap_ratio: 0.0,
            time_window: 150,
            style: SpectrogramStyle::default(),
        }
    }
}
