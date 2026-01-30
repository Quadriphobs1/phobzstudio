//! GPU-accelerated FFT using wgpu compute shaders.

use std::sync::Arc;
use wgpu::{Device, Queue};

use super::buffers::{FftBuffers, FftParamBuffers};
use super::params::{BandParams, FftParams, MagnitudeParams, WindowParams};
use super::pipelines::{FftLayouts, FftPipelines};

/// Errors that can occur during GPU FFT operations.
#[derive(Debug, thiserror::Error)]
pub enum GpuFftError {
    #[error("FFT size must be a power of 2, got {0}")]
    InvalidFftSize(usize),
    #[error("Not enough samples: need {needed} but got {got}")]
    InsufficientSamples { needed: usize, got: usize },
    #[error("Too many bands requested: max {max}, got {requested}")]
    TooManyBands { max: usize, requested: usize },
    #[error("GPU buffer mapping failed: {0}")]
    BufferMapFailed(String),
}

const MAX_BANDS: usize = 2048;
const WORKGROUP_SIZE: u32 = 256;

/// GPU-accelerated FFT analyzer.
pub struct GpuFftAnalyzer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    fft_size: usize,
    num_stages: u32,
    layouts: FftLayouts,
    pipelines: FftPipelines,
    buffers: FftBuffers,
    params: FftParamBuffers,
}

impl GpuFftAnalyzer {
    /// Create a new GPU FFT analyzer.
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        fft_size: usize,
    ) -> Result<Self, GpuFftError> {
        if !fft_size.is_power_of_two() {
            return Err(GpuFftError::InvalidFftSize(fft_size));
        }

        let num_stages = (fft_size as f32).log2() as u32;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fft_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/fft.wgsl").into()),
        });

        let layouts = FftLayouts::new(&device);
        let pipelines = FftPipelines::new(&device, &shader, &layouts);
        let buffers = FftBuffers::new(&device, fft_size, MAX_BANDS);
        let params = FftParamBuffers::new(&device);

        Ok(Self {
            device,
            queue,
            fft_size,
            num_stages,
            layouts,
            pipelines,
            buffers,
            params,
        })
    }

    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    pub fn num_bins(&self) -> usize {
        self.fft_size / 2
    }

    /// Compute magnitude spectrum from audio samples.
    pub fn analyze(&self, samples: &[f32]) -> Result<Vec<f32>, GpuFftError> {
        self.check_samples(samples)?;

        // Upload samples
        self.upload_samples(samples);

        // Step 1: Window → Bit-reverse
        {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("fft_prep_encoder"),
                });
            self.encode_window(&mut encoder);
            self.encode_bit_reverse(&mut encoder);
            self.queue.submit(Some(encoder.finish()));
        }

        // Step 2: FFT butterfly stages (each stage needs its own submit for uniform updates)
        self.run_fft_stages();

        // Step 3: Magnitude → Copy to staging
        {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("fft_mag_encoder"),
                });
            self.encode_magnitude(&mut encoder, false);
            let result_size = (self.fft_size / 2 * std::mem::size_of::<f32>()) as u64;
            encoder.copy_buffer_to_buffer(
                &self.buffers.magnitude,
                0,
                &self.buffers.staging,
                0,
                result_size,
            );
            self.queue.submit(Some(encoder.finish()));
        }

        self.read_staging(self.fft_size / 2)
    }

    /// Compute spectrum grouped into bands.
    pub fn analyze_bands(
        &self,
        samples: &[f32],
        sample_rate: u32,
        num_bands: usize,
    ) -> Result<Vec<f32>, GpuFftError> {
        self.check_samples(samples)?;
        if num_bands > MAX_BANDS {
            return Err(GpuFftError::TooManyBands {
                max: MAX_BANDS,
                requested: num_bands,
            });
        }

        // Upload samples
        self.upload_samples(samples);

        // Step 1: Window → Bit-reverse
        {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("fft_bands_prep_encoder"),
                });
            self.encode_window(&mut encoder);
            self.encode_bit_reverse(&mut encoder);
            self.queue.submit(Some(encoder.finish()));
        }

        // Step 2: FFT butterfly stages
        self.run_fft_stages();

        // Step 3: Magnitude → Bands → Copy to staging
        {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("fft_bands_mag_encoder"),
                });
            self.encode_magnitude(&mut encoder, false);
            self.encode_bands(&mut encoder, sample_rate, num_bands);
            let result_size = (num_bands * std::mem::size_of::<f32>()) as u64;
            encoder.copy_buffer_to_buffer(
                &self.buffers.bands,
                0,
                &self.buffers.staging,
                0,
                result_size,
            );
            self.queue.submit(Some(encoder.finish()));
        }

        let mut bands = self.read_staging(num_bands)?;
        Self::normalize(&mut bands);
        Ok(bands)
    }

    pub fn bin_to_freq(&self, bin: usize, sample_rate: u32) -> f32 {
        bin as f32 * sample_rate as f32 / self.fft_size as f32
    }

    pub fn freq_to_bin(&self, freq: f32, sample_rate: u32) -> usize {
        (freq * self.fft_size as f32 / sample_rate as f32).round() as usize
    }

    // --- Private helpers ---

    fn check_samples(&self, samples: &[f32]) -> Result<(), GpuFftError> {
        if samples.len() < self.fft_size {
            return Err(GpuFftError::InsufficientSamples {
                needed: self.fft_size,
                got: samples.len(),
            });
        }
        Ok(())
    }

    fn upload_samples(&self, samples: &[f32]) {
        self.queue.write_buffer(
            &self.buffers.samples,
            0,
            bytemuck::cast_slice(&samples[..self.fft_size]),
        );
    }

    fn encode_window(&self, encoder: &mut wgpu::CommandEncoder) {
        let params = WindowParams::new(self.fft_size);
        self.queue
            .write_buffer(&self.params.window, 0, bytemuck::bytes_of(&params));

        let bind_group = self.create_bind_group(
            "window",
            &self.layouts.window,
            &self.buffers.samples,
            &self.buffers.complex_a,
            &self.params.window,
        );

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("window_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipelines.window);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(self.workgroups(self.fft_size), 1, 1);
    }

    fn encode_bit_reverse(&self, encoder: &mut wgpu::CommandEncoder) {
        let params = FftParams::new(self.fft_size, 0, true);
        self.queue
            .write_buffer(&self.params.fft, 0, bytemuck::bytes_of(&params));

        let bind_group = self.create_bind_group(
            "bit_reverse",
            &self.layouts.fft,
            &self.buffers.complex_a,
            &self.buffers.complex_b,
            &self.params.fft,
        );

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("bit_reverse_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipelines.bit_reverse);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(self.workgroups(self.fft_size), 1, 1);
    }

    fn run_fft_stages(&self) {
        // Run FFT stages with separate submissions to ensure uniform buffer updates take effect.
        // After bit-reverse, data is in complex_b.
        // We ping-pong: b→a, a→b, b→a, ...
        let mut read_from_a = false;

        for stage in 0..self.num_stages {
            let params = FftParams::new(self.fft_size, stage, true);
            self.queue
                .write_buffer(&self.params.fft, 0, bytemuck::bytes_of(&params));

            let (input, output) = if read_from_a {
                (&self.buffers.complex_a, &self.buffers.complex_b)
            } else {
                (&self.buffers.complex_b, &self.buffers.complex_a)
            };

            let bind_group =
                self.create_bind_group("fft", &self.layouts.fft, input, output, &self.params.fft);

            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("fft_stage_encoder"),
                });

            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("fft_stage_pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipelines.butterfly);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(self.workgroups(self.fft_size / 2), 1, 1);
            }

            self.queue.submit(Some(encoder.finish()));
            read_from_a = !read_from_a;
        }

        // After all stages, determine where data ended up.
        // If read_from_a=true, last stage wrote to A (data in A).
        // If read_from_a=false, last stage wrote to B (need to copy to A).
        if !read_from_a {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("fft_copy_encoder"),
                });
            encoder.copy_buffer_to_buffer(
                &self.buffers.complex_b,
                0,
                &self.buffers.complex_a,
                0,
                (self.fft_size * 2 * std::mem::size_of::<f32>()) as u64,
            );
            self.queue.submit(Some(encoder.finish()));
        }
    }

    fn encode_magnitude(&self, encoder: &mut wgpu::CommandEncoder, db_mode: bool) {
        let params = MagnitudeParams::new(self.fft_size, db_mode);
        self.queue
            .write_buffer(&self.params.magnitude, 0, bytemuck::bytes_of(&params));

        let bind_group = self.create_bind_group(
            "magnitude",
            &self.layouts.magnitude,
            &self.buffers.complex_a,
            &self.buffers.magnitude,
            &self.params.magnitude,
        );

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("magnitude_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipelines.magnitude);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(self.workgroups(self.fft_size / 2), 1, 1);
    }

    fn encode_bands(&self, encoder: &mut wgpu::CommandEncoder, sample_rate: u32, num_bands: usize) {
        let params = BandParams::new(self.fft_size, num_bands, sample_rate);
        self.queue
            .write_buffer(&self.params.bands, 0, bytemuck::bytes_of(&params));

        let bind_group = self.create_bind_group(
            "bands",
            &self.layouts.bands,
            &self.buffers.magnitude,
            &self.buffers.bands,
            &self.params.bands,
        );

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("bands_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipelines.bands);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups((num_bands as u32).div_ceil(64), 1, 1);
    }

    fn create_bind_group(
        &self,
        label: &str,
        layout: &wgpu::BindGroupLayout,
        input: &wgpu::Buffer,
        output: &wgpu::Buffer,
        params: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params.as_entire_binding(),
                },
            ],
        })
    }

    fn read_staging(&self, count: usize) -> Result<Vec<f32>, GpuFftError> {
        let size = (count * std::mem::size_of::<f32>()) as u64;
        let slice = self.buffers.staging.slice(..size);

        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        rx.recv()
            .map_err(|e| GpuFftError::BufferMapFailed(e.to_string()))?
            .map_err(|e| GpuFftError::BufferMapFailed(format!("{:?}", e)))?;

        let data = slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        self.buffers.staging.unmap();

        Ok(result)
    }

    fn workgroups(&self, elements: usize) -> u32 {
        (elements as u32).div_ceil(WORKGROUP_SIZE)
    }

    fn normalize(values: &mut [f32]) {
        let max = values.iter().cloned().fold(0.0f32, f32::max);
        if max > 0.0 {
            for v in values {
                *v /= max;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> Option<(Arc<Device>, Arc<Queue>)> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .ok()?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
        Some((Arc::new(device), Arc::new(queue)))
    }

    #[test]
    fn test_creation() {
        if let Some((device, queue)) = create_test_context() {
            let analyzer = GpuFftAnalyzer::new(device, queue, 1024);
            assert!(analyzer.is_ok());
        }
    }

    #[test]
    fn test_invalid_size() {
        if let Some((device, queue)) = create_test_context() {
            let result = GpuFftAnalyzer::new(device, queue, 1000);
            assert!(matches!(result, Err(GpuFftError::InvalidFftSize(1000))));
        }
    }
}
