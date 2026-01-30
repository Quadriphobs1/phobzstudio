//! GPU spectrum buffer for zero-copy rendering.

use std::sync::Arc;
use wgpu::{Buffer, Device, Queue};

use super::fft::{GpuFftAnalyzer, GpuFftError};

/// GPU-resident spectrum buffer for direct rendering.
pub struct GpuSpectrumBuffer {
    buffer: Buffer,
    num_bands: usize,
    max_bands: usize,
}

impl GpuSpectrumBuffer {
    pub fn new(device: &Device, max_bands: usize) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("spectrum_buffer"),
            size: (max_bands * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            num_bands: 0,
            max_bands,
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
    pub fn num_bands(&self) -> usize {
        self.num_bands
    }
    pub fn max_bands(&self) -> usize {
        self.max_bands
    }

    pub fn update_from_cpu(&mut self, queue: &Queue, bands: &[f32]) {
        let count = bands.len().min(self.max_bands);
        self.num_bands = count;
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&bands[..count]));
    }
}

/// Complete GPU audio processing pipeline.
pub struct SpectrumPipeline {
    queue: Arc<Queue>,
    fft_analyzer: GpuFftAnalyzer,
    spectrum_buffer: GpuSpectrumBuffer,
    sample_rate: u32,
}

impl SpectrumPipeline {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        fft_size: usize,
        max_bands: usize,
        sample_rate: u32,
    ) -> Result<Self, GpuFftError> {
        let fft_analyzer = GpuFftAnalyzer::new(device.clone(), queue.clone(), fft_size)?;
        let spectrum_buffer = GpuSpectrumBuffer::new(&device, max_bands);

        Ok(Self {
            queue,
            fft_analyzer,
            spectrum_buffer,
            sample_rate,
        })
    }

    pub fn process(&mut self, samples: &[f32], num_bands: usize) -> Result<Vec<f32>, GpuFftError> {
        let bands = self
            .fft_analyzer
            .analyze_bands(samples, self.sample_rate, num_bands)?;
        self.spectrum_buffer.update_from_cpu(&self.queue, &bands);
        Ok(bands)
    }

    pub fn spectrum_buffer(&self) -> &GpuSpectrumBuffer {
        &self.spectrum_buffer
    }
    pub fn fft_analyzer(&self) -> &GpuFftAnalyzer {
        &self.fft_analyzer
    }
    pub fn fft_size(&self) -> usize {
        self.fft_analyzer.fft_size()
    }
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn set_sample_rate(&mut self, rate: u32) {
        self.sample_rate = rate;
    }
}

/// Builder for SpectrumPipeline.
#[derive(Default)]
pub struct SpectrumPipelineBuilder {
    fft_size: usize,
    max_bands: usize,
    sample_rate: u32,
}

impl SpectrumPipelineBuilder {
    pub fn new() -> Self {
        Self {
            fft_size: 2048,
            max_bands: 256,
            sample_rate: 44100,
        }
    }

    pub fn fft_size(mut self, size: usize) -> Self {
        self.fft_size = size;
        self
    }
    pub fn max_bands(mut self, bands: usize) -> Self {
        self.max_bands = bands;
        self
    }
    pub fn sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = rate;
        self
    }

    pub fn build(
        self,
        device: Arc<Device>,
        queue: Arc<Queue>,
    ) -> Result<SpectrumPipeline, GpuFftError> {
        SpectrumPipeline::new(
            device,
            queue,
            self.fft_size,
            self.max_bands,
            self.sample_rate,
        )
    }
}
