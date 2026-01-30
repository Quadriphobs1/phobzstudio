//! Headless waveform renderer.

use super::{
    context::GpuContext,
    pipeline::{BarInstance, WaveformPipeline, WaveformUniforms},
};
use wgpu::{BindGroup, Texture, TextureDescriptor, TextureView};

/// Configuration for rendering.
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub color: [f32; 3],
    pub background: [f32; 3],
    pub width: u32,
    pub height: u32,
    pub bar_count: u32,
    pub vertical: bool,
    pub mirror: bool,
    pub glow: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            color: [0.0, 0.8, 1.0],
            background: [0.0, 0.0, 0.0],
            width: 1920,
            height: 1080,
            bar_count: 64,
            vertical: false,
            mirror: false,
            glow: true,
        }
    }
}

/// Headless waveform renderer.
pub struct WaveformRenderer {
    ctx: GpuContext,
    pipeline: WaveformPipeline,
    bind_group: BindGroup,
    render_texture: Texture,
    render_view: TextureView,
    config: RenderConfig,
}

impl WaveformRenderer {
    /// Create a new renderer with the given configuration.
    pub async fn new(config: RenderConfig) -> Result<Self, super::context::GpuError> {
        let ctx = GpuContext::new().await?;
        let format = wgpu::TextureFormat::Rgba8Unorm;

        let pipeline = WaveformPipeline::new(&ctx.device, format, config.bar_count);
        let bind_group = pipeline.create_bind_group(&ctx.device);

        let render_texture = ctx.device.create_texture(&TextureDescriptor {
            label: Some("render_target"),
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

        Ok(Self {
            ctx,
            pipeline,
            bind_group,
            render_texture,
            render_view,
            config,
        })
    }

    /// Render a frame with the given bar heights and beat intensity.
    ///
    /// Returns RGBA pixel data.
    pub fn render_frame(&self, bar_heights: &[f32], beat_intensity: f32) -> Vec<u8> {
        let bar_count = bar_heights.len().min(self.config.bar_count as usize);

        // Update uniforms
        let uniforms = WaveformUniforms {
            width: self.config.width as f32,
            height: self.config.height as f32,
            bar_count: bar_count as f32,
            beat_intensity,
            color: self.config.color,
            layout_vertical: if self.config.vertical { 1.0 } else { 0.0 },
            mirror: if self.config.mirror { 1.0 } else { 0.0 },
            glow_enabled: if self.config.glow { 1.0 } else { 0.0 },
            _padding: [0.0; 2],
        };
        self.ctx.queue.write_buffer(
            &self.pipeline.uniform_buffer,
            0,
            bytemuck::bytes_of(&uniforms),
        );

        // Update instance data
        let instances: Vec<BarInstance> = bar_heights
            .iter()
            .take(bar_count)
            .enumerate()
            .map(|(i, &height)| BarInstance {
                height: height.clamp(0.0, 1.0),
                index: i as f32,
            })
            .collect();
        self.ctx.queue.write_buffer(
            &self.pipeline.instance_buffer,
            0,
            bytemuck::cast_slice(&instances),
        );

        // Create command encoder
        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        // Render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("waveform_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.render_view,
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

            render_pass.set_pipeline(&self.pipeline.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.pipeline.instance_buffer.slice(..));
            // Draw 4 vertices per bar (triangle strip quad)
            render_pass.draw(0..4, 0..bar_count as u32);
        }

        // Copy texture to buffer for readback
        let bytes_per_pixel = 4u32;
        let unpadded_row_bytes = self.config.width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_row_bytes = unpadded_row_bytes.div_ceil(align) * align;
        let buffer_size = (padded_row_bytes * self.config.height) as u64;

        let readback_buffer = self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback_buffer"),
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
        self.ctx
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
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
    pub fn config(&self) -> &RenderConfig {
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

    #[tokio::test]
    async fn test_renderer_creation() {
        let config = RenderConfig {
            width: 320,
            height: 180,
            bar_count: 16,
            ..Default::default()
        };

        let result = WaveformRenderer::new(config).await;
        if let Ok(renderer) = result {
            let info = renderer.adapter_info();
            assert!(!info.name.is_empty());
        }
    }

    #[tokio::test]
    async fn test_render_frame() {
        let config = RenderConfig {
            width: 320,
            height: 180,
            bar_count: 8,
            color: [1.0, 0.0, 0.0],
            background: [0.0, 0.0, 0.0],
            vertical: false,
            mirror: false,
            glow: true,
        };

        let result = WaveformRenderer::new(config.clone()).await;
        if let Ok(renderer) = result {
            let bar_heights = vec![0.5, 0.8, 0.3, 0.9, 0.4, 0.7, 0.2, 0.6];
            let pixels = renderer.render_frame(&bar_heights, 0.0);

            assert_eq!(pixels.len(), (config.width * config.height * 4) as usize);

            // Check that we have some non-black pixels (the bars)
            let has_color = pixels.chunks(4).any(|p| p[0] > 0 || p[1] > 0 || p[2] > 0);
            assert!(has_color, "Rendered frame should contain colored pixels");
        }
    }
}
