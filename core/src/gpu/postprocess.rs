//! Post-processing pipeline for bloom/glow effects.
//!
//! Implements a multi-pass bloom effect:
//! 1. Extract bright areas from rendered scene
//! 2. Apply two-pass Gaussian blur (horizontal + vertical)
//! 3. Composite blurred bloom with original scene

use wgpu::{
    BindGroupLayout, Buffer, Device, Queue, RenderPipeline, Sampler, Texture, TextureFormat,
    TextureView,
};

/// Uniform data for blur pass.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurUniforms {
    direction: [f32; 2],
    texel_size: [f32; 2],
}

/// Uniform data for bloom extraction/composition.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BloomUniforms {
    threshold: f32,
    intensity: f32,
    beat_intensity: f32,
    _padding: f32,
}

/// Configuration for the post-processing pipeline.
#[derive(Debug, Clone)]
pub struct PostProcessConfig {
    pub width: u32,
    pub height: u32,
    /// Bloom brightness threshold (0.0-1.0). Lower values bloom more.
    pub bloom_threshold: f32,
    /// Bloom intensity multiplier.
    pub bloom_intensity: f32,
    /// Number of blur passes (more = softer glow).
    pub blur_passes: u32,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            bloom_threshold: 0.5,
            bloom_intensity: 1.0,
            blur_passes: 2,
        }
    }
}

/// Post-processing pipeline for bloom/glow effects.
pub struct PostProcessPipeline {
    // Pipelines
    blur_pipeline: RenderPipeline,
    extract_pipeline: RenderPipeline,
    composite_pipeline: RenderPipeline,

    // Bind group layouts
    blur_bind_group_layout: BindGroupLayout,
    bloom_bind_group_layout: BindGroupLayout,

    // Textures (ping-pong for blur)
    bloom_texture_a: Texture,
    bloom_texture_b: Texture,
    bloom_view_a: TextureView,
    bloom_view_b: TextureView,

    // Buffers
    blur_uniform_buffer: Buffer,
    bloom_uniform_buffer: Buffer,

    // Sampler
    sampler: Sampler,

    // Configuration
    config: PostProcessConfig,
}

impl PostProcessPipeline {
    /// Create a new post-processing pipeline.
    pub fn new(device: &Device, config: PostProcessConfig) -> Self {
        let format = TextureFormat::Rgba8Unorm;

        // Create shaders
        let blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blur_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/blur.wgsl").into()),
        });

        let bloom_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/bloom.wgsl").into()),
        });

        // Blur bind group layout
        let blur_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blur_bind_group_layout"),
            entries: &[
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Input texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Bloom bind group layout (for extract and composite)
        let bloom_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_bind_group_layout"),
            entries: &[
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Scene texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Bloom texture
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Pipeline layouts
        let blur_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blur_pipeline_layout"),
            bind_group_layouts: &[&blur_bind_group_layout],
            immediate_size: 0,
        });

        let bloom_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom_pipeline_layout"),
            bind_group_layouts: &[&bloom_bind_group_layout],
            immediate_size: 0,
        });

        // Blur pipeline
        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blur_pipeline"),
            layout: Some(&blur_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blur_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blur_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Extract pipeline (extracts bright areas)
        let extract_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom_extract_pipeline"),
            layout: Some(&bloom_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &bloom_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &bloom_shader,
                entry_point: Some("fs_extract"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Composite pipeline (blends bloom with scene)
        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom_composite_pipeline"),
            layout: Some(&bloom_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &bloom_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &bloom_shader,
                entry_point: Some("fs_composite"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Create ping-pong textures for blur passes
        let texture_desc = wgpu::TextureDescriptor {
            label: Some("bloom_texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let bloom_texture_a = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bloom_texture_a"),
            ..texture_desc
        });
        let bloom_texture_b = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bloom_texture_b"),
            ..texture_desc
        });

        let bloom_view_a = bloom_texture_a.create_view(&wgpu::TextureViewDescriptor::default());
        let bloom_view_b = bloom_texture_b.create_view(&wgpu::TextureViewDescriptor::default());

        // Create uniform buffers
        let blur_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blur_uniforms"),
            size: std::mem::size_of::<BlurUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bloom_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloom_uniforms"),
            size: std::mem::size_of::<BloomUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create sampler with linear filtering for smooth blur
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        Self {
            blur_pipeline,
            extract_pipeline,
            composite_pipeline,
            blur_bind_group_layout,
            bloom_bind_group_layout,
            bloom_texture_a,
            bloom_texture_b,
            bloom_view_a,
            bloom_view_b,
            blur_uniform_buffer,
            bloom_uniform_buffer,
            sampler,
            config,
        }
    }

    /// Apply bloom post-processing to the scene texture.
    ///
    /// Returns the texture view containing the final composited result.
    pub fn apply(
        &self,
        device: &Device,
        queue: &Queue,
        encoder: &mut wgpu::CommandEncoder,
        scene_view: &TextureView,
        output_view: &TextureView,
        beat_intensity: f32,
    ) {
        // Update bloom uniforms
        let bloom_uniforms = BloomUniforms {
            threshold: self.config.bloom_threshold,
            intensity: self.config.bloom_intensity,
            beat_intensity,
            _padding: 0.0,
        };
        queue.write_buffer(&self.bloom_uniform_buffer, 0, bytemuck::bytes_of(&bloom_uniforms));

        // Step 1: Extract bright areas from scene -> bloom_texture_a
        {
            let extract_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom_extract_bind_group"),
                layout: &self.bloom_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.bloom_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(scene_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(scene_view), // Unused in extract
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_extract_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bloom_view_a,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.extract_pipeline);
            pass.set_bind_group(0, &extract_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // Step 2: Apply blur passes (ping-pong between textures)
        let texel_size = [1.0 / self.config.width as f32, 1.0 / self.config.height as f32];

        for pass_idx in 0..self.config.blur_passes {
            // Horizontal blur: a -> b
            self.blur_pass(
                device,
                queue,
                encoder,
                &self.bloom_view_a,
                &self.bloom_view_b,
                [1.0, 0.0],
                texel_size,
            );

            // Vertical blur: b -> a
            self.blur_pass(
                device,
                queue,
                encoder,
                &self.bloom_view_b,
                &self.bloom_view_a,
                [0.0, 1.0],
                texel_size,
            );

            // Increase blur radius for each pass (optional, for larger glow)
            let _ = pass_idx; // Currently unused, could scale texel_size
        }

        // Step 3: Composite bloom with original scene -> output
        {
            let composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom_composite_bind_group"),
                layout: &self.bloom_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.bloom_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(scene_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.bloom_view_a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_composite_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.composite_pipeline);
            pass.set_bind_group(0, &composite_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    /// Execute a single blur pass.
    fn blur_pass(
        &self,
        device: &Device,
        queue: &Queue,
        encoder: &mut wgpu::CommandEncoder,
        input_view: &TextureView,
        output_view: &TextureView,
        direction: [f32; 2],
        texel_size: [f32; 2],
    ) {
        // Update blur uniforms
        let blur_uniforms = BlurUniforms {
            direction,
            texel_size,
        };
        queue.write_buffer(&self.blur_uniform_buffer, 0, bytemuck::bytes_of(&blur_uniforms));

        let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur_bind_group"),
            layout: &self.blur_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.blur_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(input_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("blur_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.blur_pipeline);
        pass.set_bind_group(0, &blur_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Get the current configuration.
    pub fn config(&self) -> &PostProcessConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::GpuContext;

    #[tokio::test]
    async fn test_postprocess_pipeline_creation() {
        let ctx = match GpuContext::new().await {
            Ok(ctx) => ctx,
            Err(_) => {
                eprintln!("Skipping test - GPU not available");
                return;
            }
        };

        let config = PostProcessConfig {
            width: 256,
            height: 256,
            ..Default::default()
        };

        let _pipeline = PostProcessPipeline::new(&ctx.device, config);
    }
}
