//! Example: Render a visualization from synthetic audio.
//!
//! This example generates a synthetic beat pattern and renders a short video.
//!
//! Run with:
//!     cargo run --example render_synthetic

use phobz_visualizer::audio::synth::generate_test_beat;
use phobz_visualizer::audio::{analyze_audio, AudioData, SpectrumAnalyzer};
use phobz_visualizer::gpu::{RenderConfig, WaveformRenderer};
use phobz_visualizer::video::{VideoCodec, VideoConfig, VideoEncoder};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Phobz Visualizer - Synthetic Audio Example");
    println!("==========================================\n");

    // Generate synthetic audio (120 BPM beat, 5 seconds)
    let sample_rate: u32 = 44100;
    let duration_secs: f32 = 5.0;
    let bpm: f32 = 120.0;

    println!("Generating synthetic beat...");
    println!("  Sample rate: {} Hz", sample_rate);
    println!("  Duration: {} seconds", duration_secs);
    println!("  BPM: {}", bpm);

    let samples = generate_test_beat(bpm, sample_rate, duration_secs);
    let audio = AudioData {
        samples: samples.clone(),
        sample_rate,
        channels: 1,
    };

    println!("  Generated {} samples\n", audio.samples.len());

    // Analyze the audio
    println!("Analyzing audio...");
    let fps: f32 = 30.0;
    let num_bands: usize = 32;
    let analysis = analyze_audio(&samples, sample_rate, fps, num_bands);

    println!("  Duration: {:.2}s", analysis.duration);
    println!("  BPM: {:.1}", analysis.bpm);
    println!("  Beats detected: {}", analysis.beats.len());
    println!();

    // Setup rendering configuration
    let width = 640;
    let height = 360;
    let bar_count = 32;

    println!("Setting up renderer...");
    println!("  Resolution: {}x{}", width, height);
    println!("  FPS: {}", fps as u32);
    println!("  Bars: {}", bar_count);

    let render_config = RenderConfig {
        width,
        height,
        bar_count,
        color: [0.0, 1.0, 0.53],
        background: [0.0, 0.0, 0.0],
        vertical: false,
        mirror: false,
        glow: true,
    };

    let renderer = WaveformRenderer::new(render_config).await?;
    let adapter_info = renderer.adapter_info();
    println!("  GPU: {}\n", adapter_info.name);

    // Setup video encoder
    let output_path = Path::new("synthetic_demo.mp4");
    println!("Encoding video to: {}", output_path.display());

    let video_config = VideoConfig {
        width,
        height,
        fps: fps as u32,
        codec: VideoCodec::H264,
        bitrate: 2_000_000,
        crf: Some(23),
    };

    let mut encoder = VideoEncoder::new(output_path, video_config)?;

    // Calculate frames
    let total_frames = (duration_secs * fps).ceil() as usize;
    let samples_per_frame = sample_rate as usize / fps as usize;
    let fft_size = 2048;

    let mut analyzer = SpectrumAnalyzer::new(fft_size);

    println!("Rendering {} frames...", total_frames);

    // Render each frame
    for frame_idx in 0..total_frames {
        let time = frame_idx as f64 / fps as f64;

        // Get audio samples for this frame
        let start_sample = frame_idx * samples_per_frame;
        let end_sample = (start_sample + fft_size).min(samples.len());

        // Compute spectrum
        let bar_heights: Vec<f32> = if start_sample < samples.len()
            && end_sample - start_sample >= fft_size
        {
            let frame_samples = &samples[start_sample..end_sample];
            let spectrum = analyzer.analyze_bands(frame_samples, sample_rate, bar_count as usize);
            let max_val = spectrum.iter().cloned().fold(0.0f32, f32::max).max(0.001);
            spectrum.iter().map(|&v| (v / max_val).min(1.0)).collect()
        } else {
            vec![0.0; bar_count as usize]
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

        // Progress
        if frame_idx % 30 == 0 {
            let progress = (frame_idx + 1) as f32 / total_frames as f32 * 100.0;
            println!("  Progress: {:.0}%", progress);
        }
    }

    // Finish encoding
    encoder.finish()?;

    println!("\nDone! Output: {}", output_path.display());
    println!("Play with: ffplay {}", output_path.display());

    Ok(())
}
