//! Texture management for GPU rendering.

use wgpu::{Device, Texture, TextureFormat, TextureUsages, TextureView};

/// A render target that owns both texture and view.
/// The texture must outlive its view, so we keep them together.
pub struct RenderTarget {
    texture: Texture,
    view: TextureView,
}

impl RenderTarget {
    /// Create a new render target with the specified usage flags.
    pub fn new(
        device: &Device,
        label: &str,
        width: u32,
        height: u32,
        format: TextureFormat,
        usage: TextureUsages,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view }
    }

    /// Create a render target for scene rendering (can be sampled for post-processing).
    pub fn for_scene(
        device: &Device,
        label: &str,
        width: u32,
        height: u32,
        format: TextureFormat,
    ) -> Self {
        Self::new(
            device,
            label,
            width,
            height,
            format,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        )
    }

    /// Create a render target for final output (can be copied to CPU).
    pub fn for_output(
        device: &Device,
        label: &str,
        width: u32,
        height: u32,
        format: TextureFormat,
    ) -> Self {
        Self::new(
            device,
            label,
            width,
            height,
            format,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        )
    }

    /// Get the texture view for rendering or sampling.
    pub fn view(&self) -> &TextureView {
        &self.view
    }

    /// Get the underlying texture (for copy operations).
    pub fn texture(&self) -> &Texture {
        &self.texture
    }
}

/// Readback buffer for copying GPU texture data to CPU.
pub struct ReadbackBuffer {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_row_bytes: u32,
    unpadded_row_bytes: u32,
}

impl ReadbackBuffer {
    /// Create a new readback buffer sized for the given dimensions.
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        let bytes_per_pixel = 4u32;
        let unpadded_row_bytes = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_row_bytes = unpadded_row_bytes.div_ceil(align) * align;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback_buffer"),
            size: (padded_row_bytes * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            width,
            height,
            padded_row_bytes,
            unpadded_row_bytes,
        }
    }

    /// Get the underlying buffer.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Get the padded bytes per row (for texture copy).
    pub fn padded_row_bytes(&self) -> u32 {
        self.padded_row_bytes
    }

    /// Read pixels from the mapped buffer, removing row padding.
    pub fn read_pixels(&self, device: &wgpu::Device) -> Vec<u8> {
        let buffer_slice = self.buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        receiver.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((self.width * self.height * 4) as usize);
        for row in 0..self.height {
            let start = (row * self.padded_row_bytes) as usize;
            let end = start + self.unpadded_row_bytes as usize;
            pixels.extend_from_slice(&data[start..end]);
        }
        pixels
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::GpuContext;

    #[tokio::test]
    async fn test_render_target_creation() {
        let ctx = match GpuContext::new().await {
            Ok(ctx) => ctx,
            Err(_) => return,
        };

        let _target =
            RenderTarget::for_scene(&ctx.device, "test", 256, 256, TextureFormat::Rgba8Unorm);
        // Test passes if creation succeeds without panic
    }

    #[tokio::test]
    async fn test_readback_buffer_creation() {
        let ctx = match GpuContext::new().await {
            Ok(ctx) => ctx,
            Err(_) => return,
        };

        let buffer = ReadbackBuffer::new(&ctx.device, 256, 256);
        assert!(buffer.padded_row_bytes() >= 256 * 4);
    }
}
