//! GPU buffer management for FFT operations.

use wgpu::{Buffer, BufferUsages, Device};

/// Collection of GPU buffers used for FFT computation.
pub struct FftBuffers {
    pub samples: Buffer,
    pub complex_a: Buffer,
    pub complex_b: Buffer,
    pub magnitude: Buffer,
    pub bands: Buffer,
    pub staging: Buffer,
}

impl FftBuffers {
    /// Create all buffers needed for FFT computation.
    pub fn new(device: &Device, fft_size: usize, max_bands: usize) -> Self {
        let samples = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fft_samples_buffer"),
            size: (fft_size * std::mem::size_of::<f32>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let complex_size = (fft_size * 2 * std::mem::size_of::<f32>()) as u64;
        let complex_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fft_complex_a"),
            size: complex_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let complex_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fft_complex_b"),
            size: complex_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let magnitude = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fft_magnitude"),
            size: (fft_size / 2 * std::mem::size_of::<f32>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bands = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fft_bands"),
            size: (max_bands * std::mem::size_of::<f32>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Staging buffer large enough for both magnitude and bands
        let staging_size = (fft_size / 2).max(max_bands) * std::mem::size_of::<f32>();
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fft_staging"),
            size: staging_size as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            samples,
            complex_a,
            complex_b,
            magnitude,
            bands,
            staging,
        }
    }
}

/// Uniform parameter buffers for FFT shaders.
pub struct FftParamBuffers {
    pub window: Buffer,
    pub fft: Buffer,
    pub magnitude: Buffer,
    pub bands: Buffer,
}

impl FftParamBuffers {
    pub fn new(device: &Device) -> Self {
        Self {
            window: Self::create_uniform_buffer(device, "window_params", 32), // vec3 alignment
            fft: Self::create_uniform_buffer(device, "fft_params", 16),
            magnitude: Self::create_uniform_buffer(device, "magnitude_params", 16),
            bands: Self::create_uniform_buffer(device, "bands_params", 16),
        }
    }

    fn create_uniform_buffer(device: &Device, label: &str, size: u64) -> Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
}
