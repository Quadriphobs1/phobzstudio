//! Design registry and factory functions.
//!
//! Provides a centralized way to create design instances and get their default parameters.

use super::{
    BarsDesign, CircularRadialDesign, CircularRingDesign, Design, DesignParams, DesignType,
    FrameCornersDesign, FramePerimeterDesign, ParticlesDesign, SpectrogramDesign,
    SpectrumMountainDesign, WaveformLineDesign,
};
use super::params::*;

/// Create a design instance from type.
///
/// # Example
/// ```
/// use phobz_visualizer::designs::{create_design, DesignType};
///
/// let design = create_design(DesignType::Bars);
/// assert_eq!(design.design_type(), DesignType::Bars);
/// ```
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
        DesignType::Spectrogram => Box::new(SpectrogramDesign::new()),
    }
}

/// Get default params for a design type.
///
/// # Example
/// ```
/// use phobz_visualizer::designs::{default_params, DesignType, DesignParams};
///
/// let params = default_params(DesignType::Bars);
/// assert!(matches!(params, DesignParams::Bars(_)));
/// ```
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
        DesignType::Spectrogram => DesignParams::Spectrogram(SpectrogramParams::default()),
    }
}
