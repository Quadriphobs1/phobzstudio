//! GPU rendering using wgpu.
//!
//! Provides headless GPU rendering for waveform visualization
//! using the Metal backend on macOS.

pub mod context;
pub mod design_renderer;
pub mod layouts;
pub mod pipeline;
pub mod pipelines;
pub mod postprocess;
pub mod renderer;
pub mod textures;

pub use context::{GpuContext, GpuError};
pub use design_renderer::{DesignRenderConfig, DesignRenderer};
pub use pipeline::WaveformPipeline;
pub use postprocess::{PostProcessConfig, PostProcessPipeline};
pub use renderer::{RenderConfig, WaveformRenderer};
