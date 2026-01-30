//! Video encoder implementation using FFmpeg.

use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avformat::AVFormatContextOutput,
    avutil::{AVFrame, AVRational},
    error::RsmpegError,
    ffi,
};
use std::ffi::CString;
use std::path::Path;

/// Video codec options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodec {
    /// H.264 for YouTube/TikTok/Instagram (no transparency).
    H264,
    /// ProRes 4444 for professional workflows (supports transparency).
    ProRes4444,
    /// VP9 WebM for web use (supports transparency).
    Vp9,
}

impl VideoCodec {
    fn codec_name(&self) -> &'static str {
        match self {
            VideoCodec::H264 => "libx264",
            VideoCodec::ProRes4444 => "prores_ks",
            VideoCodec::Vp9 => "libvpx-vp9",
        }
    }

    fn pixel_format(&self) -> ffi::AVPixelFormat {
        match self {
            VideoCodec::H264 => ffi::AV_PIX_FMT_YUV420P,
            VideoCodec::ProRes4444 => ffi::AV_PIX_FMT_YUVA444P10LE,
            VideoCodec::Vp9 => ffi::AV_PIX_FMT_YUVA420P,
        }
    }
}

/// Video encoding configuration.
#[derive(Debug, Clone)]
pub struct VideoConfig {
    /// Video bitrate in bits per second (for lossy codecs).
    pub bitrate: u64,
    /// CRF quality (0-51 for H.264, lower is better). None uses bitrate.
    pub crf: Option<u32>,
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Frame rate (frames per second).
    pub fps: u32,
    /// Video codec to use.
    pub codec: VideoCodec,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            bitrate: 8_000_000, // 8 Mbps
            crf: Some(18),      // High quality
            width: 1920,
            height: 1080,
            fps: 30,
            codec: VideoCodec::H264,
        }
    }
}

/// Errors that can occur during video encoding.
#[derive(Debug, thiserror::Error)]
pub enum VideoError {
    #[error("FFmpeg error: {0}")]
    Ffmpeg(#[from] RsmpegError),
    #[error("Codec not found: {0}")]
    CodecNotFound(String),
    #[error("Failed to open output file: {0}")]
    FileOpen(String),
    #[error("Encoding error: {0}")]
    Encoding(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Video encoder for rendering frames to video files.
pub struct VideoEncoder {
    format_ctx: AVFormatContextOutput,
    codec_ctx: AVCodecContext,
    frame: AVFrame,
    config: VideoConfig,
    pts: i64,
    stream_index: i32,
}

impl VideoEncoder {
    /// Create a new video encoder.
    pub fn new<P: AsRef<Path>>(path: P, config: VideoConfig) -> Result<Self, VideoError> {
        let path_str = path.as_ref().to_string_lossy();
        let path_cstring = CString::new(path_str.as_bytes())
            .map_err(|_| VideoError::FileOpen(path_str.to_string()))?;

        // Find encoder
        let codec_name = config.codec.codec_name();
        let codec = AVCodec::find_encoder_by_name(&CString::new(codec_name).unwrap())
            .ok_or_else(|| VideoError::CodecNotFound(codec_name.to_string()))?;

        // Create format context
        let mut format_ctx = AVFormatContextOutput::create(&path_cstring)?;

        // Create codec context
        let mut codec_ctx = AVCodecContext::new(&codec);
        codec_ctx.set_width(config.width as i32);
        codec_ctx.set_height(config.height as i32);
        codec_ctx.set_pix_fmt(config.codec.pixel_format());
        codec_ctx.set_time_base(AVRational {
            num: 1,
            den: config.fps as i32,
        });
        codec_ctx.set_framerate(AVRational {
            num: config.fps as i32,
            den: 1,
        });

        // Set codec-specific options
        match config.codec {
            VideoCodec::H264 => {
                codec_ctx.set_bit_rate(config.bitrate as i64);
                if let Some(crf) = config.crf {
                    // CRF is set via private options
                    unsafe {
                        let crf_str = CString::new(crf.to_string()).unwrap();
                        let key = CString::new("crf").unwrap();
                        ffi::av_opt_set(
                            codec_ctx.as_mut_ptr() as *mut _,
                            key.as_ptr(),
                            crf_str.as_ptr(),
                            ffi::AV_OPT_SEARCH_CHILDREN as i32,
                        );
                        // Set preset to medium for good balance
                        let preset = CString::new("medium").unwrap();
                        let preset_key = CString::new("preset").unwrap();
                        ffi::av_opt_set(
                            codec_ctx.as_mut_ptr() as *mut _,
                            preset_key.as_ptr(),
                            preset.as_ptr(),
                            ffi::AV_OPT_SEARCH_CHILDREN as i32,
                        );
                    }
                }
            }
            VideoCodec::ProRes4444 => {
                // Profile 4 = ProRes 4444
                unsafe {
                    let profile = CString::new("4").unwrap();
                    let key = CString::new("profile").unwrap();
                    ffi::av_opt_set(
                        codec_ctx.as_mut_ptr() as *mut _,
                        key.as_ptr(),
                        profile.as_ptr(),
                        ffi::AV_OPT_SEARCH_CHILDREN as i32,
                    );
                }
            }
            VideoCodec::Vp9 => {
                codec_ctx.set_bit_rate(config.bitrate as i64);
                if let Some(crf) = config.crf {
                    unsafe {
                        let crf_str = CString::new(crf.to_string()).unwrap();
                        let key = CString::new("crf").unwrap();
                        ffi::av_opt_set(
                            codec_ctx.as_mut_ptr() as *mut _,
                            key.as_ptr(),
                            crf_str.as_ptr(),
                            ffi::AV_OPT_SEARCH_CHILDREN as i32,
                        );
                    }
                }
            }
        }

        // Open codec
        codec_ctx.open(None)?;

        // Add stream and get index
        let stream_index = {
            let mut stream = format_ctx.new_stream();
            stream.set_codecpar(codec_ctx.extract_codecpar());
            stream.set_time_base(codec_ctx.time_base);
            stream.index
        };

        // Write header
        format_ctx.write_header(&mut None)?;

        // Create frame
        let mut frame = AVFrame::new();
        frame.set_format(config.codec.pixel_format());
        frame.set_width(config.width as i32);
        frame.set_height(config.height as i32);
        frame.alloc_buffer()?;

        Ok(Self {
            format_ctx,
            codec_ctx,
            frame,
            pts: 0,
            stream_index,
            config,
        })
    }

    /// Write a frame of RGBA pixel data.
    ///
    /// The data should be `width * height * 4` bytes in RGBA format.
    pub fn write_frame(&mut self, rgba_data: &[u8]) -> Result<(), VideoError> {
        let expected_size = (self.config.width * self.config.height * 4) as usize;
        if rgba_data.len() != expected_size {
            return Err(VideoError::InvalidConfig(format!(
                "Expected {} bytes, got {}",
                expected_size,
                rgba_data.len()
            )));
        }

        // Convert RGBA to the target pixel format
        self.convert_rgba_to_frame(rgba_data)?;

        self.frame.set_pts(self.pts);
        self.pts += 1;

        // Encode frame
        self.codec_ctx.send_frame(Some(&self.frame))?;

        // Receive and write packets
        loop {
            let mut packet = match self.codec_ctx.receive_packet() {
                Ok(p) => p,
                Err(RsmpegError::EncoderDrainError) | Err(RsmpegError::EncoderFlushedError) => {
                    break
                }
                Err(e) => return Err(e.into()),
            };

            packet.set_stream_index(self.stream_index);
            packet.rescale_ts(
                self.codec_ctx.time_base,
                self.format_ctx
                    .streams()
                    .get(self.stream_index as usize)
                    .unwrap()
                    .time_base,
            );

            self.format_ctx.interleaved_write_frame(&mut packet)?;
        }

        Ok(())
    }

    /// Finish encoding and close the file.
    pub fn finish(mut self) -> Result<(), VideoError> {
        // Flush encoder
        self.codec_ctx.send_frame(None)?;

        loop {
            let mut packet = match self.codec_ctx.receive_packet() {
                Ok(p) => p,
                Err(RsmpegError::EncoderDrainError) | Err(RsmpegError::EncoderFlushedError) => {
                    break
                }
                Err(e) => return Err(e.into()),
            };

            packet.set_stream_index(self.stream_index);
            packet.rescale_ts(
                self.codec_ctx.time_base,
                self.format_ctx
                    .streams()
                    .get(self.stream_index as usize)
                    .unwrap()
                    .time_base,
            );

            self.format_ctx.interleaved_write_frame(&mut packet)?;
        }

        self.format_ctx.write_trailer()?;
        Ok(())
    }

    /// Get the video configuration.
    pub fn config(&self) -> &VideoConfig {
        &self.config
    }

    /// Convert RGBA data to the frame's pixel format.
    fn convert_rgba_to_frame(&mut self, rgba_data: &[u8]) -> Result<(), VideoError> {
        let width = self.config.width as usize;
        let height = self.config.height as usize;

        match self.config.codec {
            VideoCodec::H264 => {
                // Convert RGBA to YUV420P
                self.rgba_to_yuv420p(rgba_data, width, height);
            }
            VideoCodec::ProRes4444 => {
                // Convert RGBA to YUVA444P10LE
                self.rgba_to_yuva444p10(rgba_data, width, height);
            }
            VideoCodec::Vp9 => {
                // Convert RGBA to YUVA420P
                self.rgba_to_yuva420p(rgba_data, width, height);
            }
        }

        Ok(())
    }

    fn rgba_to_yuv420p(&mut self, rgba: &[u8], width: usize, height: usize) {
        let y_plane = self.frame.data[0];
        let u_plane = self.frame.data[1];
        let v_plane = self.frame.data[2];
        let y_stride = self.frame.linesize[0] as usize;
        let u_stride = self.frame.linesize[1] as usize;
        let v_stride = self.frame.linesize[2] as usize;

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                let r = rgba[idx] as f32;
                let g = rgba[idx + 1] as f32;
                let b = rgba[idx + 2] as f32;

                // BT.601 RGB to YUV
                let y_val = (0.299 * r + 0.587 * g + 0.114 * b) as u8;

                unsafe {
                    *y_plane.add(y * y_stride + x) = y_val;
                }

                // Subsample U and V (2x2 blocks)
                if x % 2 == 0 && y % 2 == 0 {
                    let u_val = (128.0 - 0.168736 * r - 0.331264 * g + 0.5 * b) as u8;
                    let v_val = (128.0 + 0.5 * r - 0.418688 * g - 0.081312 * b) as u8;

                    unsafe {
                        *u_plane.add((y / 2) * u_stride + (x / 2)) = u_val;
                        *v_plane.add((y / 2) * v_stride + (x / 2)) = v_val;
                    }
                }
            }
        }
    }

    fn rgba_to_yuva444p10(&mut self, rgba: &[u8], width: usize, height: usize) {
        let y_plane = self.frame.data[0] as *mut u16;
        let u_plane = self.frame.data[1] as *mut u16;
        let v_plane = self.frame.data[2] as *mut u16;
        let a_plane = self.frame.data[3] as *mut u16;
        let y_stride = self.frame.linesize[0] as usize / 2;
        let u_stride = self.frame.linesize[1] as usize / 2;
        let v_stride = self.frame.linesize[2] as usize / 2;
        let a_stride = self.frame.linesize[3] as usize / 2;

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                let r = rgba[idx] as f32;
                let g = rgba[idx + 1] as f32;
                let b = rgba[idx + 2] as f32;
                let a = rgba[idx + 3] as f32;

                // Scale to 10-bit (0-1023)
                let scale = 1023.0 / 255.0;

                let y_val = ((0.299 * r + 0.587 * g + 0.114 * b) * scale) as u16;
                let u_val = ((128.0 - 0.168736 * r - 0.331264 * g + 0.5 * b) * scale) as u16;
                let v_val = ((128.0 + 0.5 * r - 0.418688 * g - 0.081312 * b) * scale) as u16;
                let a_val = (a * scale) as u16;

                unsafe {
                    *y_plane.add(y * y_stride + x) = y_val;
                    *u_plane.add(y * u_stride + x) = u_val;
                    *v_plane.add(y * v_stride + x) = v_val;
                    *a_plane.add(y * a_stride + x) = a_val;
                }
            }
        }
    }

    fn rgba_to_yuva420p(&mut self, rgba: &[u8], width: usize, height: usize) {
        let y_plane = self.frame.data[0];
        let u_plane = self.frame.data[1];
        let v_plane = self.frame.data[2];
        let a_plane = self.frame.data[3];
        let y_stride = self.frame.linesize[0] as usize;
        let u_stride = self.frame.linesize[1] as usize;
        let v_stride = self.frame.linesize[2] as usize;
        let a_stride = self.frame.linesize[3] as usize;

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                let r = rgba[idx] as f32;
                let g = rgba[idx + 1] as f32;
                let b = rgba[idx + 2] as f32;
                let a = rgba[idx + 3];

                let y_val = (0.299 * r + 0.587 * g + 0.114 * b) as u8;

                unsafe {
                    *y_plane.add(y * y_stride + x) = y_val;
                    *a_plane.add(y * a_stride + x) = a;
                }

                if x % 2 == 0 && y % 2 == 0 {
                    let u_val = (128.0 - 0.168736 * r - 0.331264 * g + 0.5 * b) as u8;
                    let v_val = (128.0 + 0.5 * r - 0.418688 * g - 0.081312 * b) as u8;

                    unsafe {
                        *u_plane.add((y / 2) * u_stride + (x / 2)) = u_val;
                        *v_plane.add((y / 2) * v_stride + (x / 2)) = v_val;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_video_config_default() {
        let config = VideoConfig::default();
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.fps, 30);
        assert_eq!(config.codec, VideoCodec::H264);
    }

    #[test]
    fn test_encode_h264() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.mp4");

        let config = VideoConfig {
            width: 320,
            height: 180,
            fps: 30,
            codec: VideoCodec::H264,
            bitrate: 1_000_000,
            crf: Some(23),
        };

        let mut encoder = VideoEncoder::new(&output_path, config.clone()).unwrap();

        // Write 30 frames (1 second of video)
        let frame_data = vec![0u8; (config.width * config.height * 4) as usize];
        for _ in 0..30 {
            encoder.write_frame(&frame_data).unwrap();
        }

        encoder.finish().unwrap();

        // Check file exists and has content
        assert!(output_path.exists());
        assert!(std::fs::metadata(&output_path).unwrap().len() > 0);
    }
}
