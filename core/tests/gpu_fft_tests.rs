//! Integration tests for GPU FFT implementation.

use phobz_visualizer::audio::{DynamicAnalyzer, SpectrumAnalyze, SpectrumAnalyzer};
use phobz_visualizer::gpu::GpuContext;
use std::f32::consts::PI;

fn generate_sine(freq: f32, sample_rate: u32, num_samples: usize) -> Vec<f32> {
    (0..num_samples)
        .map(|i| (2.0 * PI * freq * i as f32 / sample_rate as f32).sin())
        .collect()
}

fn generate_composite(freqs: &[f32], sample_rate: u32, num_samples: usize) -> Vec<f32> {
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            freqs.iter().map(|&f| (2.0 * PI * f * t).sin()).sum::<f32>() / freqs.len() as f32
        })
        .collect()
}

async fn create_gpu_context() -> Option<GpuContext> {
    GpuContext::new().await.ok()
}

#[test]
fn test_cpu_fft_sine_wave() {
    let sample_rate = 44100;
    let freq = 440.0;
    let samples = generate_sine(freq, sample_rate, 4096);

    let mut analyzer = SpectrumAnalyzer::new(2048);
    let spectrum = analyzer.analyze(&samples);

    let peak_bin = spectrum
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap();

    let peak_freq = analyzer.bin_to_freq(peak_bin, sample_rate);
    assert!(
        (peak_freq - freq).abs() < 50.0,
        "Expected peak near {} Hz, got {} Hz",
        freq,
        peak_freq
    );
}

#[tokio::test]
async fn test_gpu_fft_analyzer_creation() {
    if let Some(ctx) = create_gpu_context().await {
        let analyzer =
            phobz_visualizer::gpu::GpuFftAnalyzer::new(ctx.device.clone(), ctx.queue.clone(), 1024);
        assert!(analyzer.is_ok());
        let analyzer = analyzer.unwrap();
        assert_eq!(analyzer.fft_size(), 1024);
        assert_eq!(analyzer.num_bins(), 512);
    }
}

#[tokio::test]
async fn test_gpu_fft_sine_wave() {
    if let Some(ctx) = create_gpu_context().await {
        let sample_rate = 44100;
        let freq = 440.0;
        let samples = generate_sine(freq, sample_rate, 4096);

        let analyzer =
            phobz_visualizer::gpu::GpuFftAnalyzer::new(ctx.device.clone(), ctx.queue.clone(), 2048)
                .expect("Failed to create GPU FFT analyzer");

        let spectrum = analyzer.analyze(&samples).expect("FFT analysis failed");

        let peak_bin = spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        let peak_freq = analyzer.bin_to_freq(peak_bin, sample_rate);
        assert!(
            (peak_freq - freq).abs() < 50.0,
            "Expected peak near {} Hz, got {} Hz (bin {})",
            freq,
            peak_freq,
            peak_bin
        );
    }
}

#[tokio::test]
async fn test_gpu_vs_cpu_fft_comparison() {
    if let Some(ctx) = create_gpu_context().await {
        let sample_rate = 44100;
        let samples = generate_composite(&[440.0, 880.0, 1320.0], sample_rate, 4096);

        let mut cpu_analyzer = SpectrumAnalyzer::new(2048);
        let cpu_spectrum = cpu_analyzer.analyze(&samples);

        let gpu_analyzer =
            phobz_visualizer::gpu::GpuFftAnalyzer::new(ctx.device.clone(), ctx.queue.clone(), 2048)
                .expect("Failed to create GPU FFT analyzer");

        let gpu_spectrum = gpu_analyzer.analyze(&samples).expect("GPU FFT failed");

        assert_eq!(cpu_spectrum.len(), gpu_spectrum.len());

        // Find main peaks in both
        let cpu_peak = cpu_spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        let gpu_peak = gpu_spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        let bin_diff = (cpu_peak as i32 - gpu_peak as i32).abs();
        assert!(
            bin_diff <= 5,
            "CPU peak bin {} vs GPU peak bin {}, diff {}",
            cpu_peak,
            gpu_peak,
            bin_diff
        );
    }
}

#[tokio::test]
async fn test_gpu_analyze_bands() {
    if let Some(ctx) = create_gpu_context().await {
        let sample_rate = 44100;
        let samples = generate_composite(&[200.0, 1000.0, 5000.0], sample_rate, 4096);

        let analyzer =
            phobz_visualizer::gpu::GpuFftAnalyzer::new(ctx.device.clone(), ctx.queue.clone(), 2048)
                .expect("Failed to create GPU FFT analyzer");

        let bands = analyzer
            .analyze_bands(&samples, sample_rate, 32)
            .expect("Band analysis failed");

        assert_eq!(bands.len(), 32);

        for (i, &band) in bands.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&band),
                "Band {} value {} out of range",
                i,
                band
            );
        }

        let max_band = bands.iter().cloned().fold(0.0f32, f32::max);
        assert!(
            (max_band - 1.0).abs() < 0.01,
            "Max band should be ~1.0, got {}",
            max_band
        );
    }
}

#[tokio::test]
async fn test_dynamic_analyzer_gpu() {
    if let Some(ctx) = create_gpu_context().await {
        let mut analyzer = DynamicAnalyzer::gpu(ctx.device.clone(), ctx.queue.clone(), 1024)
            .expect("Failed to create dynamic GPU analyzer");

        assert!(analyzer.is_gpu());
        assert_eq!(analyzer.fft_size(), 1024);

        let samples = generate_sine(440.0, 44100, 2048);
        let result = analyzer.analyze(&samples);
        assert!(result.is_ok());
    }
}

#[test]
fn test_dynamic_analyzer_fallback() {
    let analyzer = DynamicAnalyzer::gpu_with_fallback(None, None, 1024);
    assert!(!analyzer.is_gpu());
    assert_eq!(analyzer.fft_size(), 1024);
}

#[tokio::test]
async fn test_spectrum_pipeline() {
    if let Some(ctx) = create_gpu_context().await {
        let mut pipeline = phobz_visualizer::gpu::compute::SpectrumPipeline::new(
            ctx.device.clone(),
            ctx.queue.clone(),
            2048,
            64,
            44100,
        )
        .expect("Failed to create spectrum pipeline");

        assert_eq!(pipeline.fft_size(), 2048);
        assert_eq!(pipeline.sample_rate(), 44100);

        let samples = generate_sine(440.0, 44100, 4096);
        let bands = pipeline.process(&samples, 32).expect("Processing failed");
        assert_eq!(bands.len(), 32);
    }
}

#[tokio::test]
async fn test_spectrum_pipeline_builder() {
    if let Some(ctx) = create_gpu_context().await {
        use phobz_visualizer::gpu::compute::SpectrumPipelineBuilder;

        let pipeline = SpectrumPipelineBuilder::new()
            .fft_size(1024)
            .max_bands(128)
            .sample_rate(48000)
            .build(ctx.device.clone(), ctx.queue.clone())
            .expect("Failed to build pipeline");

        assert_eq!(pipeline.fft_size(), 1024);
        assert_eq!(pipeline.sample_rate(), 48000);
    }
}

// --- Edge case tests ---

#[tokio::test]
async fn test_gpu_fft_invalid_size() {
    if let Some(ctx) = create_gpu_context().await {
        // Non-power-of-2 sizes should fail
        let result =
            phobz_visualizer::gpu::GpuFftAnalyzer::new(ctx.device.clone(), ctx.queue.clone(), 1000);
        assert!(result.is_err());

        let result =
            phobz_visualizer::gpu::GpuFftAnalyzer::new(ctx.device.clone(), ctx.queue.clone(), 1500);
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_gpu_fft_insufficient_samples() {
    if let Some(ctx) = create_gpu_context().await {
        let analyzer =
            phobz_visualizer::gpu::GpuFftAnalyzer::new(ctx.device.clone(), ctx.queue.clone(), 2048)
                .expect("Failed to create analyzer");

        // Provide fewer samples than FFT size
        let samples = vec![0.0f32; 1024];
        let result = analyzer.analyze(&samples);
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_gpu_fft_too_many_bands() {
    if let Some(ctx) = create_gpu_context().await {
        let analyzer =
            phobz_visualizer::gpu::GpuFftAnalyzer::new(ctx.device.clone(), ctx.queue.clone(), 2048)
                .expect("Failed to create analyzer");

        let samples = generate_sine(440.0, 44100, 4096);

        // Request more bands than MAX_BANDS (2048)
        let result = analyzer.analyze_bands(&samples, 44100, 3000);
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_gpu_fft_dc_signal() {
    if let Some(ctx) = create_gpu_context().await {
        let analyzer =
            phobz_visualizer::gpu::GpuFftAnalyzer::new(ctx.device.clone(), ctx.queue.clone(), 1024)
                .expect("Failed to create analyzer");

        // DC signal (constant value) should have energy only at bin 0
        let samples = vec![0.5f32; 2048];
        let spectrum = analyzer.analyze(&samples).expect("Analysis failed");

        // Find peak - should be at or near bin 0
        let peak_bin = spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        assert!(
            peak_bin < 5,
            "DC signal peak should be near bin 0, got {}",
            peak_bin
        );
    }
}

#[tokio::test]
async fn test_gpu_fft_silence() {
    if let Some(ctx) = create_gpu_context().await {
        let analyzer =
            phobz_visualizer::gpu::GpuFftAnalyzer::new(ctx.device.clone(), ctx.queue.clone(), 1024)
                .expect("Failed to create analyzer");

        // Silent signal (all zeros)
        let samples = vec![0.0f32; 2048];
        let spectrum = analyzer.analyze(&samples).expect("Analysis failed");

        // All magnitudes should be near zero
        let max_mag = spectrum.iter().cloned().fold(0.0f32, f32::max);
        assert!(
            max_mag < 0.001,
            "Silent signal should have near-zero magnitudes, got {}",
            max_mag
        );
    }
}

#[tokio::test]
async fn test_gpu_fft_high_frequency() {
    if let Some(ctx) = create_gpu_context().await {
        let sample_rate = 44100;
        let freq = 10000.0; // 10 kHz
        let samples = generate_sine(freq, sample_rate, 4096);

        let analyzer =
            phobz_visualizer::gpu::GpuFftAnalyzer::new(ctx.device.clone(), ctx.queue.clone(), 2048)
                .expect("Failed to create analyzer");

        let spectrum = analyzer.analyze(&samples).expect("Analysis failed");

        let peak_bin = spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        let peak_freq = analyzer.bin_to_freq(peak_bin, sample_rate);
        assert!(
            (peak_freq - freq).abs() < 100.0,
            "Expected peak near {} Hz, got {} Hz",
            freq,
            peak_freq
        );
    }
}

#[tokio::test]
async fn test_gpu_fft_various_sizes() {
    if let Some(ctx) = create_gpu_context().await {
        let samples = generate_sine(1000.0, 44100, 8192);

        for fft_size in [256, 512, 1024, 2048, 4096] {
            let analyzer = phobz_visualizer::gpu::GpuFftAnalyzer::new(
                ctx.device.clone(),
                ctx.queue.clone(),
                fft_size,
            )
            .expect(&format!("Failed to create analyzer with size {}", fft_size));

            let spectrum = analyzer
                .analyze(&samples)
                .expect(&format!("Analysis failed for size {}", fft_size));

            assert_eq!(
                spectrum.len(),
                fft_size / 2,
                "Spectrum length mismatch for size {}",
                fft_size
            );
        }
    }
}

#[tokio::test]
async fn test_gpu_fft_large_band_count() {
    if let Some(ctx) = create_gpu_context().await {
        let samples = generate_sine(440.0, 44100, 4096);

        let analyzer =
            phobz_visualizer::gpu::GpuFftAnalyzer::new(ctx.device.clone(), ctx.queue.clone(), 2048)
                .expect("Failed to create analyzer");

        // Test with large but valid band counts
        for num_bands in [256, 512, 1024, 2048] {
            let bands = analyzer
                .analyze_bands(&samples, 44100, num_bands)
                .expect(&format!("Failed with {} bands", num_bands));

            assert_eq!(bands.len(), num_bands);

            // All bands should be normalized (0-1)
            for (i, &band) in bands.iter().enumerate() {
                assert!(
                    (0.0..=1.0).contains(&band),
                    "Band {} = {} out of range for {} bands",
                    i,
                    band,
                    num_bands
                );
            }
        }
    }
}

#[tokio::test]
async fn test_gpu_fft_repeated_analysis() {
    if let Some(ctx) = create_gpu_context().await {
        let analyzer =
            phobz_visualizer::gpu::GpuFftAnalyzer::new(ctx.device.clone(), ctx.queue.clone(), 1024)
                .expect("Failed to create analyzer");

        // Run multiple analyses to ensure no state leakage
        for freq in [220.0, 440.0, 880.0, 1760.0] {
            let samples = generate_sine(freq, 44100, 2048);
            let spectrum = analyzer
                .analyze(&samples)
                .expect(&format!("Analysis failed for {} Hz", freq));

            let peak_bin = spectrum
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap();

            let peak_freq = analyzer.bin_to_freq(peak_bin, 44100);
            assert!(
                (peak_freq - freq).abs() < 50.0,
                "Expected {} Hz, got {} Hz",
                freq,
                peak_freq
            );
        }
    }
}
