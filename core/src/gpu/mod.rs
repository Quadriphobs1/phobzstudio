//! GPU rendering using wgpu.
//!
//! Provides headless GPU rendering for waveform visualization
//! using the Metal backend on macOS.

pub mod context;
pub mod design_renderer;
pub mod pipeline;
pub mod renderer;

pub use context::{GpuContext, GpuError};
pub use design_renderer::{DesignRenderConfig, DesignRenderer};
pub use pipeline::WaveformPipeline;
pub use renderer::{RenderConfig, WaveformRenderer};
