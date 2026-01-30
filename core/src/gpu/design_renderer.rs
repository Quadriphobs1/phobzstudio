//! Design-based renderer supporting multiple visualization styles.

use super::context::GpuContext;
use super::layouts::BindGroupLayoutBuilder;
use super::pipelines::{create_pipeline_layout, RenderPipelineBuilder};
use super::postprocess::{PostProcessConfig, PostProcessPipeline};
use super::textures::{ReadbackBuffer, RenderTarget};
use crate::designs::{
    create_design, default_params, Design, DesignConfig, DesignParams, DesignType, Vertex,
};
use wgpu::{BindGroup, Buffer, RenderPipeline, ShaderStages, TextureFormat, TextureView};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DesignUniforms {
    color: [f32; 4],
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

/// Glow-enabled rendering resources (only allocated when glow is enabled).
struct GlowResources {
    scene_target: RenderTarget,
    postprocess: PostProcessPipeline,
}

/// Design-based renderer supporting multiple visualization styles.
pub struct DesignRenderer {
    ctx: GpuContext,
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    uniform_buffer: Buffer,
    vertex_buffer: Buffer,
    output_target: RenderTarget,
    glow: Option<GlowResources>,
    config: DesignRenderConfig,
    design: Box<dyn Design>,
    max_vertices: usize,
}

impl DesignRenderer {
    /// Create a new design renderer.
    pub async fn new(config: DesignRenderConfig) -> Result<Self, super::context::GpuError> {
        let ctx = GpuContext::new().await?;
        let format = TextureFormat::Rgba8Unorm;

        let shader = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("design_shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/design.wgsl").into()),
            });

        let bind_group_layout = BindGroupLayoutBuilder::new("design_bind_group_layout")
            .uniform(0, ShaderStages::VERTEX | ShaderStages::FRAGMENT)
            .build(&ctx.device);

        let pipeline_layout =
            create_pipeline_layout(&ctx.device, "design_pipeline_layout", &[&bind_group_layout]);

        let pipeline = RenderPipelineBuilder::new("design_pipeline")
            .layout(&pipeline_layout)
            .shader(&shader)
            .vertex_buffers(vec![Self::vertex_buffer_layout()])
            .format(format)
            .blend(wgpu::BlendState::ALPHA_BLENDING)
            .build(&ctx.device);

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

        // Output texture (always needed for readback)
        let output_target = RenderTarget::for_output(
            &ctx.device,
            "design_output",
            config.width,
            config.height,
            format,
        );

        // Glow resources (only when enabled)
        let glow = if config.glow {
            Some(GlowResources {
                scene_target: RenderTarget::for_scene(
                    &ctx.device,
                    "design_scene",
                    config.width,
                    config.height,
                    format,
                ),
                postprocess: PostProcessPipeline::new(
                    &ctx.device,
                    PostProcessConfig {
                        width: config.width,
                        height: config.height,
                        bloom_threshold: 0.3,
                        bloom_intensity: 1.2,
                        blur_passes: 2,
                    },
                ),
            })
        } else {
            None
        };

        let design = create_design(config.design_type);

        Ok(Self {
            ctx,
            pipeline,
            bind_group,
            uniform_buffer,
            vertex_buffer,
            output_target,
            glow,
            config,
            design,
            max_vertices,
        })
    }

    fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
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
        }
    }

    /// Render a frame with the given spectrum data and beat intensity.
    pub fn render_frame(&self, spectrum: &[f32], beat_intensity: f32) -> Vec<u8> {
        let vertices = self.design.generate_vertices(
            spectrum,
            &DesignConfig {
                width: self.config.width,
                height: self.config.height,
                color: self.config.color,
                background: self.config.background,
                bar_count: self.config.bar_count,
                glow: self.config.glow,
                beat_intensity,
            },
            &self.config.design_params,
        );

        let vertex_count = vertices.len().min(self.max_vertices);

        // Update GPU state
        self.ctx.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&DesignUniforms {
                color: [
                    self.config.color[0],
                    self.config.color[1],
                    self.config.color[2],
                    1.0,
                ],
                beat_intensity,
                glow_enabled: if self.config.glow { 1.0 } else { 0.0 },
                _padding: [0.0; 2],
            }),
        );

        if !vertices.is_empty() {
            self.ctx.queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&vertices[..vertex_count]),
            );
        }

        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render"),
            });

        // Determine render target
        let (render_target, needs_postprocess) = match &self.glow {
            Some(glow) => (glow.scene_target.view(), true),
            None => (self.output_target.view(), false),
        };

        // Main render pass
        self.run_render_pass(&mut encoder, render_target, vertex_count);

        // Post-process if glow enabled
        if needs_postprocess {
            if let Some(glow) = &self.glow {
                glow.postprocess.apply(
                    &self.ctx.device,
                    &self.ctx.queue,
                    &mut encoder,
                    glow.scene_target.view(),
                    self.output_target.view(),
                    beat_intensity,
                );
            }
        }

        // Readback
        let readback = ReadbackBuffer::new(&self.ctx.device, self.config.width, self.config.height);
        self.copy_to_buffer(&mut encoder, &readback);
        self.ctx.queue.submit(std::iter::once(encoder.finish()));
        readback.read_pixels(&self.ctx.device)
    }

    fn run_render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &TextureView,
        vertex_count: usize,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("design_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
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

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..vertex_count as u32, 0..1);
    }

    fn copy_to_buffer(&self, encoder: &mut wgpu::CommandEncoder, readback: &ReadbackBuffer) {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: self.output_target.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: readback.buffer(),
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(readback.padded_row_bytes()),
                    rows_per_image: Some(self.config.height),
                },
            },
            wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn config(&self) -> &DesignRenderConfig {
        &self.config
    }

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
        for (design_type, design_params) in [
            (DesignType::Bars, DesignParams::Bars(BarsParams::default())),
            (
                DesignType::CircularRadial,
                DesignParams::CircularRadial(CircularRadialParams::default()),
            ),
            (
                DesignType::CircularRing,
                DesignParams::CircularRing(CircularRingParams::default()),
            ),
        ] {
            with_renderer(
                DesignRenderConfig {
                    width: 256,
                    height: 256,
                    bar_count: 32,
                    design_type,
                    design_params,
                    ..Default::default()
                },
                |renderer, config| {
                    let pixels = renderer
                        .render_frame(&(0..32).map(|i| i as f32 / 32.0).collect::<Vec<_>>(), 0.5);
                    assert_eq!(pixels.len(), (config.width * config.height * 4) as usize);
                    assert!(pixels.iter().any(|&p| p > 0));
                },
            )
            .await;
        }
    }

    #[tokio::test]
    async fn test_background_color_rendering() {
        with_renderer(
            DesignRenderConfig {
                width: 64,
                height: 64,
                bar_count: 4,
                background: [0.0, 0.0, 0.0],
                glow: false,
                ..Default::default()
            },
            |r, _| {
                let p = r.render_frame(&vec![0.0; 4], 0.0);
                assert!(p[0] < 10 && p[1] < 10 && p[2] < 10);
            },
        )
        .await;

        with_renderer(
            DesignRenderConfig {
                width: 64,
                height: 64,
                bar_count: 4,
                background: [1.0, 1.0, 1.0],
                glow: false,
                ..Default::default()
            },
            |r, _| {
                let p = r.render_frame(&vec![0.0; 4], 0.0);
                assert!(p[0] > 240 && p[1] > 240 && p[2] > 240);
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_beat_intensity_changes_output() {
        with_renderer(
            DesignRenderConfig {
                width: 128,
                height: 128,
                bar_count: 8,
                glow: false,
                ..Default::default()
            },
            |r, _| {
                assert_ne!(
                    r.render_frame(&vec![0.5; 8], 0.0),
                    r.render_frame(&vec![0.5; 8], 1.0)
                );
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_glow_changes_output() {
        match (
            DesignRenderer::new(DesignRenderConfig {
                width: 128,
                height: 128,
                bar_count: 8,
                glow: true,
                ..Default::default()
            })
            .await,
            DesignRenderer::new(DesignRenderConfig {
                width: 128,
                height: 128,
                bar_count: 8,
                glow: false,
                ..Default::default()
            })
            .await,
        ) {
            (Ok(r1), Ok(r2)) => assert_ne!(
                r1.render_frame(&vec![0.8; 8], 0.0),
                r2.render_frame(&vec![0.8; 8], 0.0)
            ),
            _ => eprintln!("Skipping test - GPU not available"),
        }
    }

    #[tokio::test]
    async fn test_multiple_frames_consistent() {
        with_renderer(
            DesignRenderConfig {
                width: 128,
                height: 128,
                bar_count: 16,
                ..Default::default()
            },
            |r, c| {
                for i in 0..5 {
                    assert_eq!(
                        r.render_frame(
                            &(0..16)
                                .map(|j| ((i + j) as f32 / 20.0).min(1.0))
                                .collect::<Vec<_>>(),
                            i as f32 / 5.0
                        )
                        .len(),
                        (c.width * c.height * 4) as usize
                    );
                }
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_empty_spectrum_renders() {
        with_renderer(
            DesignRenderConfig {
                width: 64,
                height: 64,
                bar_count: 8,
                ..Default::default()
            },
            |r, c| {
                assert_eq!(
                    r.render_frame(&[], 0.0).len(),
                    (c.width * c.height * 4) as usize
                );
            },
        )
        .await;
    }
}
