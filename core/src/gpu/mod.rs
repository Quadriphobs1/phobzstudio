//! GPU rendering using wgpu.
//!
//! Provides headless GPU rendering for waveform visualization
//! using the Metal backend on macOS.

pub mod context;
pub mod pipeline;
pub mod renderer;

pub use context::{GpuContext, GpuError};
pub use pipeline::WaveformPipeline;
pub use renderer::{RenderConfig, WaveformRenderer};
