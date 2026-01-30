//! Audio loading, analysis, and processing.
//!
//! This module provides:
//! - Audio file loading via Symphonia (WAV, MP3, FLAC, AAC)
//! - FFT spectrum analysis via RustFFT (CPU) or wgpu compute shaders (GPU)
//! - Beat detection and BPM estimation
//! - RMS energy envelope calculation
//! - Unified analyzer trait for CPU/GPU abstraction

pub mod analysis;
pub mod analyzer;
pub mod fft;
pub mod loader;
pub mod synth;

// Re-export commonly used types
pub use analysis::{analyze_audio, detect_beats, estimate_bpm, AudioAnalysis, BeatInfo};
pub use analyzer::{AnalyzerError, DynamicAnalyzer, GpuAnalyzerWrapper, SpectrumAnalyze};
pub use fft::SpectrumAnalyzer;
pub use loader::{load_audio, AudioData, AudioError};
pub use synth::{
    generate_click_track, generate_kick, generate_sine, generate_test_beat, generate_white_noise,
};
