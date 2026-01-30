//! Design-based renderer supporting multiple visualization styles.

use super::context::GpuContext;
use super::postprocess::{PostProcessConfig, PostProcessPipeline};
use crate::designs::{create_design, default_params, Design, DesignConfig, DesignParams, DesignType, Vertex};
use wgpu::{BindGroup, Buffer, RenderPipeline, Texture, TextureView};

/// Uniform data for design shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DesignUniforms {
    color: [f32; 4],       // rgb + unused alpha (vec4 for alignment)
    beat_intensity: f32,
    glow_enabled: f32,
    _padding: [f32; 2],
}

/// Configuration for design-based rendering.
#[derive(Debug, Clone)]
pub struct DesignRenderConfig {
    pub width: u32,
    pub height: u32,
    pub color: [f32; 3],
    pub background: [f32; 3],
    pub bar_count: u32,
    pub glow: bool,
    pub design_type: DesignType,
    pub design_params: DesignParams,
}

impl Default for DesignRenderConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            color: [0.0, 1.0, 0.53],
            background: [0.0, 0.0, 0.0],
            bar_count: 64,
            glow: true,
            design_type: DesignType::Bars,
            design_params: default_params(DesignType::Bars),
        }
    }
}

/// Design-based renderer supporting multiple visualization styles.
pub struct DesignRenderer {
    ctx: GpuContext,
    pipeline: RenderPipeline,
    #[allow(dead_code)]
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: BindGroup,
    uniform_buffer: Buffer,
    vertex_buffer: Buffer,
    // Scene texture (for post-processing input)
    scene_texture: Texture,
    scene_view: TextureView,
    // Output texture (final result)
    render_texture: Texture,
    render_view: TextureView,
    // Post-processing pipeline (optional, used when glow is enabled)
    postprocess: Option<PostProcessPipeline>,
    config: DesignRenderConfig,
    design: Box<dyn Design>,
    max_vertices: usize,
}

impl DesignRenderer {
    /// Create a new design renderer.
    pub async fn new(config: DesignRenderConfig) -> Result<Self, super::context::GpuError> {
        let ctx = GpuContext::new().await?;
        let format = wgpu::TextureFormat::Rgba8Unorm;

        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("design_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/design.wgsl").into()),
        });

        let bind_group_layout = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("design_bind_group_layout"),
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

        let pipeline_layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("design_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let pipeline = ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("design_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: 20,
                            shader_location: 3,
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
                topology: wgpu::PrimitiveTopology::TriangleList,
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

        let uniform_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("design_uniforms"),
            size: std::mem::size_of::<DesignUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let max_vertices = config.bar_count as usize * 6;
        let vertex_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("design_vertices"),
            size: (std::mem::size_of::<Vertex>() * max_vertices) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("design_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Scene texture (for post-processing input, needs TEXTURE_BINDING for sampling)
        let scene_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("design_scene_texture"),
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
        });

        let scene_view = scene_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Output texture (final result)
        let render_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("design_render_target"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create post-processing pipeline if glow is enabled
        let postprocess = if config.glow {
            Some(PostProcessPipeline::new(
                &ctx.device,
                PostProcessConfig {
                    width: config.width,
                    height: config.height,
                    bloom_threshold: 0.3,
                    bloom_intensity: 1.2,
                    blur_passes: 2,
                },
            ))
        } else {
            None
        };

        let design = create_design(config.design_type);

        Ok(Self {
            ctx,
            pipeline,
            bind_group_layout,
            bind_group,
            uniform_buffer,
            vertex_buffer,
            scene_texture,
            scene_view,
            render_texture,
            render_view,
            postprocess,
            config,
            design,
            max_vertices,
        })
    }

    /// Render a frame with the given spectrum data and beat intensity.
    pub fn render_frame(&self, spectrum: &[f32], beat_intensity: f32) -> Vec<u8> {
        // Create design config
        let design_config = DesignConfig {
            width: self.config.width,
            height: self.config.height,
            color: self.config.color,
            background: self.config.background,
            bar_count: self.config.bar_count,
            glow: self.config.glow,
            beat_intensity,
        };

        // Generate vertices using design
        let vertices = self.design.generate_vertices(spectrum, &design_config, &self.config.design_params);
        let vertex_count = vertices.len().min(self.max_vertices);

        // Update uniforms
        let uniforms = DesignUniforms {
            color: [
                self.config.color[0],
                self.config.color[1],
                self.config.color[2],
                1.0, // unused alpha
            ],
            beat_intensity,
            glow_enabled: if self.config.glow { 1.0 } else { 0.0 },
            _padding: [0.0; 2],
        };
        self.ctx.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&uniforms),
        );

        // Update vertex buffer
        if !vertices.is_empty() {
            self.ctx.queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&vertices[..vertex_count]),
            );
        }

        // Create command encoder
        let mut encoder = self.ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("design_render_encoder"),
        });

        // Render scene to scene_view (or directly to render_view if no post-processing)
        let target_view = if self.postprocess.is_some() {
            &self.scene_view
        } else {
            &self.render_view
        };

        // Render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("design_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.config.background[0] as f64,
                            g: self.config.background[1] as f64,
                            b: self.config.background[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..vertex_count as u32, 0..1);
        }

        // Apply post-processing if enabled
        if let Some(postprocess) = &self.postprocess {
            postprocess.apply(
                &self.ctx.device,
                &self.ctx.queue,
                &mut encoder,
                &self.scene_view,
                &self.render_view,
                beat_intensity,
            );
        }

        // Copy texture to buffer for readback
        let bytes_per_pixel = 4u32;
        let unpadded_row_bytes = self.config.width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_row_bytes = unpadded_row_bytes.div_ceil(align) * align;
        let buffer_size = (padded_row_bytes * self.config.height) as u64;

        let readback_buffer = self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("design_readback_buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes),
                    rows_per_image: Some(self.config.height),
                },
            },
            wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
        );

        self.ctx.queue.submit(std::iter::once(encoder.finish()));

        // Read back pixels
        let buffer_slice = readback_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        self.ctx.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        receiver.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();

        // Remove row padding if present
        let mut pixels = Vec::with_capacity((self.config.width * self.config.height * 4) as usize);
        for row in 0..self.config.height {
            let start = (row * padded_row_bytes) as usize;
            let end = start + unpadded_row_bytes as usize;
            pixels.extend_from_slice(&data[start..end]);
        }

        pixels
    }

    /// Get the render configuration.
    pub fn config(&self) -> &DesignRenderConfig {
        &self.config
    }

    /// Get GPU adapter info.
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.ctx.adapter_info()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::designs::{BarsParams, CircularRadialParams, CircularRingParams};

    async fn with_renderer<F>(config: DesignRenderConfig, test_fn: F)
    where
        F: FnOnce(&DesignRenderer, &DesignRenderConfig),
    {
        match DesignRenderer::new(config.clone()).await {
            Ok(renderer) => test_fn(&renderer, &config),
            Err(e) => eprintln!("Skipping test - GPU not available: {}", e),
        }
    }

    #[tokio::test]
    async fn test_all_design_types_render_correct_size() {
        let design_configs = [
            (DesignType::Bars, DesignParams::Bars(BarsParams::default())),
            (DesignType::CircularRadial, DesignParams::CircularRadial(CircularRadialParams::default())),
            (DesignType::CircularRing, DesignParams::CircularRing(CircularRingParams::default())),
        ];

        for (design_type, design_params) in design_configs {
            let config = DesignRenderConfig {
                width: 256,
                height: 256,
                bar_count: 32,
                design_type,
                design_params,
                ..Default::default()
            };

            with_renderer(config, |renderer, config| {
                let spectrum: Vec<f32> = (0..32).map(|i| i as f32 / 32.0).collect();
                let pixels = renderer.render_frame(&spectrum, 0.5);

                assert_eq!(pixels.len(), (config.width * config.height * 4) as usize);
                assert!(pixels.iter().any(|&p| p > 0), "Design {:?} rendered nothing", config.design_type);
            }).await;
        }
    }

    #[tokio::test]
    async fn test_background_color_rendering() {
        // Black background
        let config_black = DesignRenderConfig {
            width: 64,
            height: 64,
            bar_count: 4,
            background: [0.0, 0.0, 0.0],
            glow: false,
            ..Default::default()
        };

        with_renderer(config_black, |renderer, _| {
            let pixels = renderer.render_frame(&vec![0.0; 4], 0.0);
            // Corners should be black
            assert!(pixels[0] < 10 && pixels[1] < 10 && pixels[2] < 10);
        }).await;

        // White background
        let config_white = DesignRenderConfig {
            width: 64,
            height: 64,
            bar_count: 4,
            background: [1.0, 1.0, 1.0],
            glow: false,
            ..Default::default()
        };

        with_renderer(config_white, |renderer, _| {
            let pixels = renderer.render_frame(&vec![0.0; 4], 0.0);
            // Corners should be white
            assert!(pixels[0] > 240 && pixels[1] > 240 && pixels[2] > 240);
        }).await;
    }

    #[tokio::test]
    async fn test_beat_intensity_changes_output() {
        let config = DesignRenderConfig {
            width: 128,
            height: 128,
            bar_count: 8,
            glow: false,
            ..Default::default()
        };

        with_renderer(config, |renderer, _| {
            let spectrum: Vec<f32> = vec![0.5; 8];
            let no_beat = renderer.render_frame(&spectrum, 0.0);
            let full_beat = renderer.render_frame(&spectrum, 1.0);
            assert_ne!(no_beat, full_beat);
        }).await;
    }

    #[tokio::test]
    async fn test_glow_changes_output() {
        let config_glow = DesignRenderConfig {
            width: 128,
            height: 128,
            bar_count: 8,
            glow: true,
            ..Default::default()
        };
        let config_no_glow = DesignRenderConfig { glow: false, ..config_glow.clone() };

        match (DesignRenderer::new(config_glow).await, DesignRenderer::new(config_no_glow).await) {
            (Ok(r1), Ok(r2)) => {
                let spectrum: Vec<f32> = vec![0.8; 8];
                assert_ne!(r1.render_frame(&spectrum, 0.0), r2.render_frame(&spectrum, 0.0));
            }
            _ => eprintln!("Skipping test - GPU not available"),
        }
    }

    #[tokio::test]
    async fn test_multiple_frames_consistent() {
        let config = DesignRenderConfig {
            width: 128,
            height: 128,
            bar_count: 16,
            ..Default::default()
        };

        with_renderer(config, |renderer, config| {
            let expected_size = (config.width * config.height * 4) as usize;
            for i in 0..5 {
                let spectrum: Vec<f32> = (0..16).map(|j| ((i + j) as f32 / 20.0).min(1.0)).collect();
                assert_eq!(renderer.render_frame(&spectrum, i as f32 / 5.0).len(), expected_size);
            }
        }).await;
    }

    #[tokio::test]
    async fn test_empty_spectrum_renders() {
        let config = DesignRenderConfig {
            width: 64,
            height: 64,
            bar_count: 8,
            ..Default::default()
        };

        with_renderer(config, |renderer, config| {
            let pixels = renderer.render_frame(&[], 0.0);
            assert_eq!(pixels.len(), (config.width * config.height * 4) as usize);
        }).await;
    }
}
