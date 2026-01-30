//! Post-processing pipeline for bloom/glow effects.
//!
//! Implements a multi-pass bloom effect:
//! 1. Extract bright areas from rendered scene
//! 2. Apply two-pass Gaussian blur (horizontal + vertical)
//! 3. Composite blurred bloom with original scene

use super::layouts::{create_bloom_layout, create_blur_layout};
use super::pipelines::{create_fullscreen_pipeline, create_pipeline_layout};
use super::textures::RenderTarget;
use wgpu::{
    BindGroupLayout, Buffer, Device, Queue, RenderPipeline, Sampler, TextureFormat, TextureView,
};

/// Uniform data for blur pass.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurUniforms {
    direction: [f32; 2],
    texel_size: [f32; 2],
}

/// Input/output texture views for a blur pass.
struct BlurPassViews<'a> {
    input: &'a TextureView,
    output: &'a TextureView,
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
    pub bloom_threshold: f32,
    pub bloom_intensity: f32,
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
    blur_pipeline: RenderPipeline,
    extract_pipeline: RenderPipeline,
    composite_pipeline: RenderPipeline,
    blur_bind_group_layout: BindGroupLayout,
    bloom_bind_group_layout: BindGroupLayout,
    bloom_target_a: RenderTarget,
    bloom_target_b: RenderTarget,
    blur_uniform_buffer: Buffer,
    bloom_uniform_buffer: Buffer,
    sampler: Sampler,
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

        // Create bind group layouts
        let blur_bind_group_layout = create_blur_layout(device);
        let bloom_bind_group_layout = create_bloom_layout(device);

        // Create pipeline layouts
        let blur_pipeline_layout =
            create_pipeline_layout(device, "blur_pipeline_layout", &[&blur_bind_group_layout]);
        let bloom_pipeline_layout =
            create_pipeline_layout(device, "bloom_pipeline_layout", &[&bloom_bind_group_layout]);

        // Create pipelines
        let blur_pipeline = create_fullscreen_pipeline(
            device,
            "blur_pipeline",
            &blur_pipeline_layout,
            &blur_shader,
            "fs_main",
            format,
        );
        let extract_pipeline = create_fullscreen_pipeline(
            device,
            "bloom_extract_pipeline",
            &bloom_pipeline_layout,
            &bloom_shader,
            "fs_extract",
            format,
        );
        let composite_pipeline = create_fullscreen_pipeline(
            device,
            "bloom_composite_pipeline",
            &bloom_pipeline_layout,
            &bloom_shader,
            "fs_composite",
            format,
        );

        // Create ping-pong render targets for blur
        let bloom_target_a = RenderTarget::for_scene(
            device,
            "bloom_texture_a",
            config.width,
            config.height,
            format,
        );
        let bloom_target_b = RenderTarget::for_scene(
            device,
            "bloom_texture_b",
            config.width,
            config.height,
            format,
        );

        // Create uniform buffers
        let blur_uniform_buffer =
            Self::create_uniform_buffer::<BlurUniforms>(device, "blur_uniforms");
        let bloom_uniform_buffer =
            Self::create_uniform_buffer::<BloomUniforms>(device, "bloom_uniforms");

        // Create sampler
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
            bloom_target_a,
            bloom_target_b,
            blur_uniform_buffer,
            bloom_uniform_buffer,
            sampler,
            config,
        }
    }

    fn create_uniform_buffer<T: bytemuck::Pod>(device: &Device, label: &'static str) -> Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: std::mem::size_of::<T>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Apply bloom post-processing to the scene texture.
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
        queue.write_buffer(
            &self.bloom_uniform_buffer,
            0,
            bytemuck::bytes_of(&BloomUniforms {
                threshold: self.config.bloom_threshold,
                intensity: self.config.bloom_intensity,
                beat_intensity,
                _padding: 0.0,
            }),
        );

        // Extract bright areas -> bloom_target_a
        self.run_bloom_pass(
            device,
            encoder,
            &self.extract_pipeline,
            scene_view,
            scene_view,
            self.bloom_target_a.view(),
        );

        // Blur passes (ping-pong)
        let texel_size = [
            1.0 / self.config.width as f32,
            1.0 / self.config.height as f32,
        ];
        for _ in 0..self.config.blur_passes {
            // Horizontal blur pass
            self.run_blur_pass(
                device,
                queue,
                encoder,
                BlurPassViews {
                    input: self.bloom_target_a.view(),
                    output: self.bloom_target_b.view(),
                },
                BlurUniforms {
                    direction: [1.0, 0.0],
                    texel_size,
                },
            );
            // Vertical blur pass
            self.run_blur_pass(
                device,
                queue,
                encoder,
                BlurPassViews {
                    input: self.bloom_target_b.view(),
                    output: self.bloom_target_a.view(),
                },
                BlurUniforms {
                    direction: [0.0, 1.0],
                    texel_size,
                },
            );
        }

        // Composite bloom with scene -> output
        self.run_bloom_pass(
            device,
            encoder,
            &self.composite_pipeline,
            scene_view,
            self.bloom_target_a.view(),
            output_view,
        );
    }

    fn run_bloom_pass(
        &self,
        device: &Device,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &RenderPipeline,
        scene_view: &TextureView,
        bloom_view: &TextureView,
        output_view: &TextureView,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
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
                    resource: wgpu::BindingResource::TextureView(bloom_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
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

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    fn run_blur_pass(
        &self,
        device: &Device,
        queue: &Queue,
        encoder: &mut wgpu::CommandEncoder,
        views: BlurPassViews<'_>,
        uniforms: BlurUniforms,
    ) {
        queue.write_buffer(&self.blur_uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.blur_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.blur_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(views.input),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: views.output,
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
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

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
            Err(_) => return,
        };

        let _pipeline = PostProcessPipeline::new(
            &ctx.device,
            PostProcessConfig {
                width: 256,
                height: 256,
                ..Default::default()
            },
        );
    }
}
