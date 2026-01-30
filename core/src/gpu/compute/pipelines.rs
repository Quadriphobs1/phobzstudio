//! Compute pipeline creation for FFT operations.

use wgpu::{BindGroupLayout, ComputePipeline, Device, ShaderModule};

/// All compute pipelines needed for FFT processing.
pub struct FftPipelines {
    pub window: ComputePipeline,
    pub bit_reverse: ComputePipeline,
    pub butterfly: ComputePipeline,
    pub magnitude: ComputePipeline,
    pub bands: ComputePipeline,
}

/// All bind group layouts for FFT pipelines.
pub struct FftLayouts {
    pub window: BindGroupLayout,
    pub fft: BindGroupLayout,
    pub magnitude: BindGroupLayout,
    pub bands: BindGroupLayout,
}

impl FftLayouts {
    /// Create all bind group layouts.
    pub fn new(device: &Device) -> Self {
        Self {
            window: Self::create_storage_uniform_layout(device, "window", true),
            fft: Self::create_storage_uniform_layout(device, "fft", false),
            magnitude: Self::create_storage_uniform_layout(device, "magnitude", false),
            bands: Self::create_storage_uniform_layout(device, "bands", false),
        }
    }

    /// Create a standard layout: input storage, output storage, uniform params.
    fn create_storage_uniform_layout(
        device: &Device,
        name: &str,
        _input_f32: bool,
    ) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{}_layout", name)),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }
}

impl FftPipelines {
    /// Create all compute pipelines from the shader module.
    pub fn new(device: &Device, shader: &ShaderModule, layouts: &FftLayouts) -> Self {
        Self {
            window: Self::create_pipeline(device, shader, &layouts.window, "apply_window"),
            bit_reverse: Self::create_pipeline(device, shader, &layouts.fft, "bit_reverse_permute"),
            butterfly: Self::create_pipeline(device, shader, &layouts.fft, "fft_butterfly"),
            magnitude: Self::create_pipeline(
                device,
                shader,
                &layouts.magnitude,
                "compute_magnitude",
            ),
            bands: Self::create_pipeline(device, shader, &layouts.bands, "compute_bands"),
        }
    }

    fn create_pipeline(
        device: &Device,
        shader: &ShaderModule,
        layout: &BindGroupLayout,
        entry_point: &str,
    ) -> ComputePipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{}_pipeline_layout", entry_point)),
            bind_group_layouts: &[layout],
            immediate_size: 0,
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(&format!("{}_pipeline", entry_point)),
            layout: Some(&pipeline_layout),
            module: shader,
            entry_point: Some(entry_point),
            compilation_options: Default::default(),
            cache: None,
        })
    }
}
