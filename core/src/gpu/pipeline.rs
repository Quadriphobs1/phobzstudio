//! Waveform rendering pipeline.

use wgpu::{BindGroup, BindGroupLayout, Buffer, Device, RenderPipeline, TextureFormat};

/// Uniform data passed to shaders.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WaveformUniforms {
    pub width: f32,
    pub height: f32,
    pub bar_count: f32,
    pub beat_intensity: f32,
    pub color: [f32; 3],
    pub layout_vertical: f32,
    pub mirror: f32,
    pub glow_enabled: f32,
    pub _padding: [f32; 2],
}

/// Per-bar instance data.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BarInstance {
    /// Bar height (0.0 - 1.0 normalized).
    pub height: f32,
    /// Bar index.
    pub index: f32,
}

/// Waveform rendering pipeline.
pub struct WaveformPipeline {
    pub pipeline: RenderPipeline,
    pub bind_group_layout: BindGroupLayout,
    pub uniform_buffer: Buffer,
    pub instance_buffer: Buffer,
}

impl WaveformPipeline {
    /// Create a new waveform pipeline.
    pub fn new(device: &Device, format: TextureFormat, max_bars: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("waveform_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/waveform.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("waveform_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("waveform_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("waveform_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<BarInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: 4,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("waveform_uniforms"),
            size: std::mem::size_of::<WaveformUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("waveform_instances"),
            size: (std::mem::size_of::<BarInstance>() * max_bars as usize) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group_layout,
            uniform_buffer,
            instance_buffer,
        }
    }

    /// Create a bind group for this pipeline.
    pub fn create_bind_group(&self, device: &Device) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("waveform_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.uniform_buffer.as_entire_binding(),
            }],
        })
    }
}
