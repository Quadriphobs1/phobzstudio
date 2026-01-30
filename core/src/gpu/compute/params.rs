//! Uniform parameter structs for FFT shaders.
//!
//! These structs must match the WGSL shader definitions exactly,
//! including alignment requirements.

/// FFT computation parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FftParams {
    pub n: u32,
    pub stage: u32,
    pub direction: i32,
    pub log2_n: u32,
}

impl FftParams {
    pub fn new(fft_size: usize, stage: u32, forward: bool) -> Self {
        Self {
            n: fft_size as u32,
            stage,
            direction: if forward { 1 } else { -1 },
            log2_n: (fft_size as f32).log2() as u32,
        }
    }
}

/// Window function parameters.
/// WGSL: struct WindowParams { n: u32, _padding: vec3<u32> }
/// vec3 has 16-byte alignment, so struct total is 32 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WindowParams {
    pub n: u32,
    pub _pad1: u32,
    pub _pad2: u32,
    pub _pad3: u32,
    // vec3<u32> takes 12 bytes but aligned at 16, total struct 32 bytes
    pub _vec3_x: u32,
    pub _vec3_y: u32,
    pub _vec3_z: u32,
    pub _vec3_pad: u32,
}

impl WindowParams {
    pub fn new(fft_size: usize) -> Self {
        Self {
            n: fft_size as u32,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
            _vec3_x: 0,
            _vec3_y: 0,
            _vec3_z: 0,
            _vec3_pad: 0,
        }
    }
}

/// Magnitude computation parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MagnitudeParams {
    pub n: u32,
    pub scale: f32,
    pub db_mode: u32,
    pub _padding: u32,
}

impl MagnitudeParams {
    pub fn new(fft_size: usize, db_mode: bool) -> Self {
        Self {
            n: fft_size as u32,
            scale: 1.0 / (fft_size as f32).sqrt(),
            db_mode: if db_mode { 1 } else { 0 },
            _padding: 0,
        }
    }
}

/// Band grouping parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BandParams {
    pub num_bins: u32,
    pub num_bands: u32,
    pub sample_rate: u32,
    pub min_freq: f32,
}

impl BandParams {
    pub fn new(fft_size: usize, num_bands: usize, sample_rate: u32) -> Self {
        Self {
            num_bins: (fft_size / 2) as u32,
            num_bands: num_bands as u32,
            sample_rate,
            min_freq: 20.0,
        }
    }
}
