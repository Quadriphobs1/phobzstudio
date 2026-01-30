//! Render pipeline builders for GPU rendering.
//!
//! Provides reusable helpers for creating wgpu render pipelines.

use wgpu::{
    BindGroupLayout, ColorTargetState, Device, PipelineLayout, RenderPipeline, ShaderModule,
    TextureFormat, VertexBufferLayout,
};

/// Builder for creating render pipelines with common patterns.
pub struct RenderPipelineBuilder<'a> {
    label: Option<&'static str>,
    layout: Option<&'a PipelineLayout>,
    shader: Option<&'a ShaderModule>,
    vertex_entry: &'static str,
    fragment_entry: &'static str,
    vertex_buffers: Vec<VertexBufferLayout<'static>>,
    format: TextureFormat,
    blend: Option<wgpu::BlendState>,
}

impl<'a> RenderPipelineBuilder<'a> {
    /// Create a new render pipeline builder.
    pub fn new(label: &'static str) -> Self {
        Self {
            label: Some(label),
            layout: None,
            shader: None,
            vertex_entry: "vs_main",
            fragment_entry: "fs_main",
            vertex_buffers: Vec::new(),
            format: TextureFormat::Rgba8Unorm,
            blend: Some(wgpu::BlendState::REPLACE),
        }
    }

    /// Set the pipeline layout.
    pub fn layout(mut self, layout: &'a PipelineLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Set the shader module.
    pub fn shader(mut self, shader: &'a ShaderModule) -> Self {
        self.shader = Some(shader);
        self
    }

    /// Set custom entry points.
    pub fn entry_points(mut self, vertex: &'static str, fragment: &'static str) -> Self {
        self.vertex_entry = vertex;
        self.fragment_entry = fragment;
        self
    }

    /// Set the fragment shader entry point only.
    pub fn fragment_entry(mut self, entry: &'static str) -> Self {
        self.fragment_entry = entry;
        self
    }

    /// Set vertex buffer layouts.
    pub fn vertex_buffers(mut self, buffers: Vec<VertexBufferLayout<'static>>) -> Self {
        self.vertex_buffers = buffers;
        self
    }

    /// Set the texture format.
    pub fn format(mut self, format: TextureFormat) -> Self {
        self.format = format;
        self
    }

    /// Set the blend state.
    pub fn blend(mut self, blend: wgpu::BlendState) -> Self {
        self.blend = Some(blend);
        self
    }

    /// Build the render pipeline.
    pub fn build(self, device: &Device) -> RenderPipeline {
        let shader = self.shader.expect("Shader module required");

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: self.label,
            layout: self.layout,
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some(self.vertex_entry),
                buffers: &self.vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some(self.fragment_entry),
                targets: &[Some(ColorTargetState {
                    format: self.format,
                    blend: self.blend,
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
        })
    }
}

/// Create a pipeline layout from bind group layouts.
pub fn create_pipeline_layout(
    device: &Device,
    label: &'static str,
    layouts: &[&BindGroupLayout],
) -> PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: layouts,
        immediate_size: 0,
    })
}

/// Create a fullscreen quad pipeline (no vertex buffers, draws 3 vertices).
pub fn create_fullscreen_pipeline(
    device: &Device,
    label: &'static str,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    fragment_entry: &'static str,
    format: TextureFormat,
) -> RenderPipeline {
    RenderPipelineBuilder::new(label)
        .layout(layout)
        .shader(shader)
        .fragment_entry(fragment_entry)
        .format(format)
        .build(device)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::GpuContext;

    #[tokio::test]
    async fn test_pipeline_layout_creation() {
        let ctx = match GpuContext::new().await {
            Ok(ctx) => ctx,
            Err(_) => return,
        };

        let bind_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("test"),
                    entries: &[],
                });

        let _layout = create_pipeline_layout(&ctx.device, "test_layout", &[&bind_group_layout]);
    }
}
