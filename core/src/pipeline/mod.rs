//! Full render pipeline combining audio, GPU, and video.

use crate::audio::{load_audio, AudioAnalysis, AudioError, DynamicAnalyzer, SpectrumAnalyze};
use crate::designs::{default_params, BarsParams, DesignParams, DesignType};
use crate::gpu::{DesignRenderConfig, DesignRenderer, GpuContext, GpuError, RenderConfig};
use crate::video::{VideoCodec, VideoConfig, VideoEncoder, VideoError};
use std::path::Path;

/// Pipeline configuration for rendering audio visualizations to video.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub bitrate: u64,
    pub fft_size: usize,
    pub color: [f32; 3],
    pub background: [f32; 3],
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bar_count: u32,
    pub codec: VideoCodec,
    pub mirror: bool,
    pub glow: bool,
    pub design_type: DesignType,
    /// Use GPU-accelerated FFT for spectrum analysis.
    /// When enabled, FFT computation happens on the GPU compute shaders.
    pub use_gpu_fft: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            bitrate: 8_000_000,
            fft_size: 2048,
            color: [0.0, 1.0, 0.53],
            background: [0.0, 0.0, 0.0],
            width: 1920,
            height: 1080,
            fps: 30,
            bar_count: 64,
            codec: VideoCodec::H264,
            mirror: false,
            glow: true,
            design_type: DesignType::Bars,
            use_gpu_fft: false,
        }
    }
}

impl PipelineConfig {
    pub fn to_render_config(&self) -> RenderConfig {
        RenderConfig {
            color: self.color,
            background: self.background,
            width: self.width,
            height: self.height,
            bar_count: self.bar_count,
            vertical: self.height > self.width,
            mirror: self.mirror,
            glow: self.glow,
        }
    }

    /// Convert to DesignRenderConfig for design-based rendering.
    pub fn to_design_render_config(&self) -> DesignRenderConfig {
        let design_params = match self.design_type {
            DesignType::Bars => DesignParams::Bars(BarsParams {
                mirror: self.mirror,
                gap_ratio: 0.1,
                vertical: self.height > self.width,
            }),
            _ => default_params(self.design_type),
        };

        DesignRenderConfig {
            width: self.width,
            height: self.height,
            color: self.color,
            background: self.background,
            bar_count: self.bar_count,
            glow: self.glow,
            design_type: self.design_type,
            design_params,
        }
    }

    /// Convert to VideoConfig for encoding.
    pub fn to_video_config(&self) -> VideoConfig {
        VideoConfig {
            bitrate: self.bitrate,
            crf: None,
            width: self.width,
            height: self.height,
            fps: self.fps,
            codec: self.codec,
        }
    }
}

/// Errors that can occur during pipeline execution.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Audio error: {0}")]
    Audio(#[from] AudioError),
    #[error("GPU error: {0}")]
    Gpu(#[from] GpuError),
    #[error("Video error: {0}")]
    Video(#[from] VideoError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Parse hex color to RGB floats (accepts 6-char RGB or 8-char RGBA, alpha is ignored).
pub fn parse_hex_color(hex: &str) -> Option<[f32; 3]> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 && hex.len() != 8 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
    Some([r, g, b])
}

/// Analyze audio file and return analysis data.
pub fn analyze_audio_file<P: AsRef<Path>>(audio_path: P) -> Result<AudioAnalysis, PipelineError> {
    let audio = load_audio(audio_path.as_ref())?;
    let mono = audio.to_mono();
    let analysis = crate::audio::analyze_audio(&mono, audio.sample_rate, 30.0, 64);
    Ok(analysis)
}

/// Render visualization video from audio file.
pub async fn render_video<P: AsRef<Path>, Q: AsRef<Path>>(
    audio_path: P,
    output_path: Q,
    config: PipelineConfig,
    progress_callback: Option<Box<dyn Fn(f32) + Send>>,
) -> Result<(), PipelineError> {
    // Load audio
    let audio = load_audio(audio_path.as_ref())?;
    let mono = audio.to_mono();

    // Analyze for beat detection
    let analysis = crate::audio::analyze_audio(
        &mono,
        audio.sample_rate,
        config.fps as f32,
        config.bar_count as usize,
    );

    // Calculate total frames
    let total_frames = (audio.duration() * config.fps as f64).ceil() as usize;
    let samples_per_frame = audio.sample_rate as usize / config.fps as usize;

    // Create GPU context (needed for both rendering and optionally GPU FFT)
    let gpu_context = GpuContext::new().await?;

    // Create spectrum analyzer (CPU or GPU based on config)
    let mut analyzer: DynamicAnalyzer = if config.use_gpu_fft {
        DynamicAnalyzer::gpu_with_fallback(
            Some(gpu_context.device.clone()),
            Some(gpu_context.queue.clone()),
            config.fft_size,
        )
    } else {
        DynamicAnalyzer::cpu(config.fft_size)
    };

    // Log which analyzer is being used
    if analyzer.is_gpu() {
        log::info!("Using GPU-accelerated FFT for spectrum analysis");
    } else {
        log::info!("Using CPU-based FFT for spectrum analysis");
    }

    // Create GPU renderer using design system
    let renderer = DesignRenderer::new(config.to_design_render_config()).await?;

    // Create video encoder using config conversion
    let mut encoder = VideoEncoder::new(output_path.as_ref(), config.to_video_config())?;

    // Render each frame
    for frame_idx in 0..total_frames {
        let time = frame_idx as f64 / config.fps as f64;

        // Get audio samples for this frame
        let start_sample = frame_idx * samples_per_frame;
        let end_sample = (start_sample + config.fft_size).min(mono.len());

        // Compute spectrum using the unified analyzer interface
        let bar_heights = if start_sample < mono.len() {
            let samples = &mono[start_sample..end_sample.min(mono.len())];
            if samples.len() >= config.fft_size {
                match analyzer.analyze_bands(samples, audio.sample_rate, config.bar_count as usize)
                {
                    Ok(spectrum) => {
                        // Normalize spectrum to 0-1 range
                        let max_val = spectrum.iter().cloned().fold(0.0f32, f32::max).max(0.001);
                        spectrum.iter().map(|&v| (v / max_val).min(1.0)).collect()
                    }
                    Err(_) => vec![0.0; config.bar_count as usize],
                }
            } else {
                vec![0.0; config.bar_count as usize]
            }
        } else {
            vec![0.0; config.bar_count as usize]
        };

        // Calculate beat intensity
        let beat_intensity = analysis
            .beats
            .iter()
            .map(|b| {
                let diff = (time - b.time).abs();
                if diff < 0.1 {
                    (1.0 - diff * 10.0) as f32
                } else {
                    0.0
                }
            })
            .fold(0.0f32, f32::max);

        // Render frame
        let pixels = renderer.render_frame(&bar_heights, beat_intensity);

        // Encode frame
        encoder.write_frame(&pixels)?;

        // Report progress
        if let Some(ref callback) = progress_callback {
            callback((frame_idx + 1) as f32 / total_frames as f32);
        }
    }

    // Finish encoding
    encoder.finish()?;

    Ok(())
}

/// Render visualization video with explicit GPU FFT enabled.
///
/// This is a convenience function that enables GPU-accelerated FFT processing.
pub async fn render_video_gpu<P: AsRef<Path>, Q: AsRef<Path>>(
    audio_path: P,
    output_path: Q,
    mut config: PipelineConfig,
    progress_callback: Option<Box<dyn Fn(f32) + Send>>,
) -> Result<(), PipelineError> {
    config.use_gpu_fft = true;
    render_video(audio_path, output_path, config, progress_callback).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#00ff88"), Some([0.0, 1.0, 136.0 / 255.0]));
        assert_eq!(parse_hex_color("ffffff"), Some([1.0, 1.0, 1.0]));
        assert_eq!(parse_hex_color("000000"), Some([0.0, 0.0, 0.0]));
        assert_eq!(parse_hex_color("#00000000"), Some([0.0, 0.0, 0.0]));
        assert_eq!(parse_hex_color("ffffffff"), Some([1.0, 1.0, 1.0]));
        assert_eq!(parse_hex_color("invalid"), None);
    }

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.fps, 30);
        assert_eq!(config.bar_count, 64);
    }
}
