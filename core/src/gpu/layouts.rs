//! Bind group layout builders for GPU pipelines.
//!
//! Provides reusable helpers for creating wgpu bind group layouts.

use wgpu::{BindGroupLayout, BindGroupLayoutEntry, Device, ShaderStages};

/// Builder for creating bind group layouts with common patterns.
pub struct BindGroupLayoutBuilder {
    label: Option<&'static str>,
    entries: Vec<BindGroupLayoutEntry>,
}

impl BindGroupLayoutBuilder {
    /// Create a new bind group layout builder.
    pub fn new(label: &'static str) -> Self {
        Self {
            label: Some(label),
            entries: Vec::new(),
        }
    }

    /// Add a uniform buffer entry.
    pub fn uniform(mut self, binding: u32, visibility: ShaderStages) -> Self {
        self.entries.push(BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self
    }

    /// Add a 2D texture entry.
    pub fn texture_2d(mut self, binding: u32, visibility: ShaderStages) -> Self {
        self.entries.push(BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        });
        self
    }

    /// Add a filtering sampler entry.
    pub fn sampler(mut self, binding: u32, visibility: ShaderStages) -> Self {
        self.entries.push(BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        self
    }

    /// Build the bind group layout.
    pub fn build(self, device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: self.label,
            entries: &self.entries,
        })
    }
}

/// Create blur bind group layout (uniforms, texture, sampler).
pub fn create_blur_layout(device: &Device) -> BindGroupLayout {
    BindGroupLayoutBuilder::new("blur_bind_group_layout")
        .uniform(0, ShaderStages::FRAGMENT)
        .texture_2d(1, ShaderStages::FRAGMENT)
        .sampler(2, ShaderStages::FRAGMENT)
        .build(device)
}

/// Create bloom bind group layout (uniforms, scene texture, bloom texture, sampler).
pub fn create_bloom_layout(device: &Device) -> BindGroupLayout {
    BindGroupLayoutBuilder::new("bloom_bind_group_layout")
        .uniform(0, ShaderStages::FRAGMENT)
        .texture_2d(1, ShaderStages::FRAGMENT)
        .texture_2d(2, ShaderStages::FRAGMENT)
        .sampler(3, ShaderStages::FRAGMENT)
        .build(device)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::GpuContext;

    #[tokio::test]
    async fn test_bind_group_layout_builder() {
        let ctx = match GpuContext::new().await {
            Ok(ctx) => ctx,
            Err(_) => return, // Skip if no GPU
        };

        let layout = BindGroupLayoutBuilder::new("test_layout")
            .uniform(0, ShaderStages::VERTEX)
            .texture_2d(1, ShaderStages::FRAGMENT)
            .sampler(2, ShaderStages::FRAGMENT)
            .build(&ctx.device);

        // Layout should be created without panicking
        drop(layout);
    }

    #[tokio::test]
    async fn test_blur_layout_creation() {
        let ctx = match GpuContext::new().await {
            Ok(ctx) => ctx,
            Err(_) => return,
        };

        let _layout = create_blur_layout(&ctx.device);
    }

    #[tokio::test]
    async fn test_bloom_layout_creation() {
        let ctx = match GpuContext::new().await {
            Ok(ctx) => ctx,
            Err(_) => return,
        };

        let _layout = create_bloom_layout(&ctx.device);
    }
}
