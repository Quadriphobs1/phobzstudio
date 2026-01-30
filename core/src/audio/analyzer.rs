//! Unified spectrum analyzer trait for CPU and GPU implementations.
//!
//! This module provides a common interface for spectrum analysis that can be
//! implemented by both CPU-based (RustFFT) and GPU-based (wgpu compute) analyzers.

use std::sync::Arc;
use wgpu::{Device, Queue};

/// Error type for spectrum analysis operations.
#[derive(Debug, thiserror::Error)]
pub enum AnalyzerError {
    #[error("FFT size must be a power of 2, got {0}")]
    InvalidFftSize(usize),
    #[error("Not enough samples: need {needed} but got {got}")]
    InsufficientSamples { needed: usize, got: usize },
    #[error("GPU error: {0}")]
    GpuError(String),
}

/// Trait for spectrum analyzers that can compute frequency-domain data from audio samples.
pub trait SpectrumAnalyze {
    /// FFT size being used.
    fn fft_size(&self) -> usize;

    /// Number of frequency bins in the output (FFT size / 2).
    fn num_bins(&self) -> usize {
        self.fft_size() / 2
    }

    /// Compute magnitude spectrum from audio samples.
    ///
    /// Returns magnitudes for frequencies from 0 to Nyquist (sample_rate / 2).
    fn analyze(&mut self, samples: &[f32]) -> Result<Vec<f32>, AnalyzerError>;

    /// Compute spectrum grouped into bands for visualization.
    ///
    /// Groups frequency bins into `num_bands` logarithmically-spaced bands.
    fn analyze_bands(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        num_bands: usize,
    ) -> Result<Vec<f32>, AnalyzerError>;

    /// Get the frequency in Hz for a given bin index.
    fn bin_to_freq(&self, bin: usize, sample_rate: u32) -> f32 {
        bin as f32 * sample_rate as f32 / self.fft_size() as f32
    }

    /// Get the bin index for a given frequency in Hz.
    fn freq_to_bin(&self, freq: f32, sample_rate: u32) -> usize {
        (freq * self.fft_size() as f32 / sample_rate as f32).round() as usize
    }
}

/// Implement the trait for the CPU-based SpectrumAnalyzer
impl SpectrumAnalyze for super::fft::SpectrumAnalyzer {
    fn fft_size(&self) -> usize {
        self.fft_size()
    }

    fn analyze(&mut self, samples: &[f32]) -> Result<Vec<f32>, AnalyzerError> {
        if samples.len() < self.fft_size() {
            return Err(AnalyzerError::InsufficientSamples {
                needed: self.fft_size(),
                got: samples.len(),
            });
        }
        Ok(super::fft::SpectrumAnalyzer::analyze(self, samples))
    }

    fn analyze_bands(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        num_bands: usize,
    ) -> Result<Vec<f32>, AnalyzerError> {
        if samples.len() < self.fft_size() {
            return Err(AnalyzerError::InsufficientSamples {
                needed: self.fft_size(),
                got: samples.len(),
            });
        }
        Ok(super::fft::SpectrumAnalyzer::analyze_bands(
            self,
            samples,
            sample_rate,
            num_bands,
        ))
    }
}

/// Wrapper around GpuFftAnalyzer that implements SpectrumAnalyze trait.
///
/// This allows using the GPU analyzer interchangeably with the CPU analyzer.
pub struct GpuAnalyzerWrapper {
    inner: crate::gpu::compute::fft::GpuFftAnalyzer,
}

impl GpuAnalyzerWrapper {
    /// Create a new GPU analyzer wrapper.
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        fft_size: usize,
    ) -> Result<Self, AnalyzerError> {
        let inner = crate::gpu::compute::fft::GpuFftAnalyzer::new(device, queue, fft_size)
            .map_err(|e| AnalyzerError::GpuError(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Get a reference to the underlying GPU analyzer.
    pub fn inner(&self) -> &crate::gpu::compute::fft::GpuFftAnalyzer {
        &self.inner
    }
}

impl SpectrumAnalyze for GpuAnalyzerWrapper {
    fn fft_size(&self) -> usize {
        self.inner.fft_size()
    }

    fn analyze(&mut self, samples: &[f32]) -> Result<Vec<f32>, AnalyzerError> {
        self.inner.analyze(samples).map_err(|e| match e {
            crate::gpu::compute::fft::GpuFftError::InsufficientSamples { needed, got } => {
                AnalyzerError::InsufficientSamples { needed, got }
            }
            crate::gpu::compute::fft::GpuFftError::InvalidFftSize(size) => {
                AnalyzerError::InvalidFftSize(size)
            }
            other => AnalyzerError::GpuError(other.to_string()),
        })
    }

    fn analyze_bands(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        num_bands: usize,
    ) -> Result<Vec<f32>, AnalyzerError> {
        self.inner
            .analyze_bands(samples, sample_rate, num_bands)
            .map_err(|e| match e {
                crate::gpu::compute::fft::GpuFftError::InsufficientSamples { needed, got } => {
                    AnalyzerError::InsufficientSamples { needed, got }
                }
                crate::gpu::compute::fft::GpuFftError::InvalidFftSize(size) => {
                    AnalyzerError::InvalidFftSize(size)
                }
                other => AnalyzerError::GpuError(other.to_string()),
            })
    }
}

/// Enum to hold either CPU or GPU analyzer for runtime selection.
pub enum DynamicAnalyzer {
    Cpu(super::fft::SpectrumAnalyzer),
    Gpu(Box<GpuAnalyzerWrapper>),
}

impl DynamicAnalyzer {
    /// Create a CPU-based analyzer.
    pub fn cpu(fft_size: usize) -> Self {
        DynamicAnalyzer::Cpu(super::fft::SpectrumAnalyzer::new(fft_size))
    }

    /// Create a GPU-based analyzer.
    pub fn gpu(
        device: Arc<Device>,
        queue: Arc<Queue>,
        fft_size: usize,
    ) -> Result<Self, AnalyzerError> {
        Ok(DynamicAnalyzer::Gpu(Box::new(GpuAnalyzerWrapper::new(
            device, queue, fft_size,
        )?)))
    }

    /// Try to create a GPU analyzer, falling back to CPU if GPU is unavailable.
    pub fn gpu_with_fallback(
        device: Option<Arc<Device>>,
        queue: Option<Arc<Queue>>,
        fft_size: usize,
    ) -> Self {
        match (device, queue) {
            (Some(device), Some(queue)) => match GpuAnalyzerWrapper::new(device, queue, fft_size) {
                Ok(gpu) => DynamicAnalyzer::Gpu(Box::new(gpu)),
                Err(_) => DynamicAnalyzer::Cpu(super::fft::SpectrumAnalyzer::new(fft_size)),
            },
            _ => DynamicAnalyzer::Cpu(super::fft::SpectrumAnalyzer::new(fft_size)),
        }
    }

    /// Check if this analyzer is using the GPU.
    pub fn is_gpu(&self) -> bool {
        matches!(self, DynamicAnalyzer::Gpu(_))
    }
}

impl SpectrumAnalyze for DynamicAnalyzer {
    fn fft_size(&self) -> usize {
        match self {
            DynamicAnalyzer::Cpu(a) => SpectrumAnalyze::fft_size(a),
            DynamicAnalyzer::Gpu(a) => a.fft_size(),
        }
    }

    fn analyze(&mut self, samples: &[f32]) -> Result<Vec<f32>, AnalyzerError> {
        match self {
            DynamicAnalyzer::Cpu(a) => SpectrumAnalyze::analyze(a, samples),
            DynamicAnalyzer::Gpu(a) => a.analyze(samples),
        }
    }

    fn analyze_bands(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        num_bands: usize,
    ) -> Result<Vec<f32>, AnalyzerError> {
        match self {
            DynamicAnalyzer::Cpu(a) => {
                SpectrumAnalyze::analyze_bands(a, samples, sample_rate, num_bands)
            }
            DynamicAnalyzer::Gpu(a) => a.analyze_bands(samples, sample_rate, num_bands),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_analyzer_trait() {
        let mut analyzer = super::super::fft::SpectrumAnalyzer::new(1024);
        assert_eq!(analyzer.fft_size(), 1024);
        assert_eq!(analyzer.num_bins(), 512);

        // Generate test signal
        let samples: Vec<f32> = (0..1024)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin())
            .collect();

        let result = SpectrumAnalyze::analyze(&mut analyzer, &samples);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 512);
    }

    #[test]
    fn test_dynamic_analyzer_cpu() {
        let mut analyzer = DynamicAnalyzer::cpu(1024);
        assert!(!analyzer.is_gpu());
        assert_eq!(analyzer.fft_size(), 1024);

        let samples: Vec<f32> = (0..1024)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin())
            .collect();

        let result = analyzer.analyze(&samples);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dynamic_analyzer_fallback() {
        // Without GPU context, should fall back to CPU
        let analyzer = DynamicAnalyzer::gpu_with_fallback(None, None, 1024);
        assert!(!analyzer.is_gpu());
    }
}
