//! GPU compute shader modules for audio processing.
//!
//! This module provides GPU-accelerated audio processing using wgpu compute shaders.

mod buffers;
mod params;
mod pipelines;

pub mod fft;
pub mod spectrum;

pub use fft::{GpuFftAnalyzer, GpuFftError};
pub use spectrum::{GpuSpectrumBuffer, SpectrumPipeline, SpectrumPipelineBuilder};
