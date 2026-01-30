//! Integration tests for audio module.

use phobz_visualizer::audio::{
    analyze_audio, detect_beats, estimate_bpm, generate_click_track, generate_sine,
    generate_test_beat, generate_white_noise, SpectrumAnalyzer,
};

const SAMPLE_RATE: u32 = 44100;

#[test]
fn test_sine_wave_spectrum_peak() {
    // Generate a 1kHz sine wave
    let freq = 1000.0;
    let samples = generate_sine(freq, SAMPLE_RATE, 1.0, 1.0);

    let mut analyzer = SpectrumAnalyzer::new(2048);
    let spectrum = analyzer.analyze(&samples);

    // Find the peak frequency
    let peak_bin = spectrum
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap();

    let peak_freq = analyzer.bin_to_freq(peak_bin, SAMPLE_RATE);

    // Peak should be within 50 Hz of expected frequency
    assert!(
        (peak_freq - freq).abs() < 50.0,
        "Expected peak at {} Hz, got {} Hz",
        freq,
        peak_freq
    );
}

#[test]
fn test_multiple_frequencies() {
    // Generate two tones
    let freq1 = 440.0;
    let freq2 = 880.0;
    let samples1 = generate_sine(freq1, SAMPLE_RATE, 1.0, 0.5);
    let samples2 = generate_sine(freq2, SAMPLE_RATE, 1.0, 0.5);

    // Mix them
    let mixed: Vec<f32> = samples1.iter().zip(&samples2).map(|(a, b)| a + b).collect();

    let mut analyzer = SpectrumAnalyzer::new(4096);
    let spectrum = analyzer.analyze(&mixed);

    // Find peaks
    let bin1 = analyzer.freq_to_bin(freq1, SAMPLE_RATE);
    let bin2 = analyzer.freq_to_bin(freq2, SAMPLE_RATE);

    // Both frequencies should have significant energy
    assert!(spectrum[bin1] > 0.1, "Expected energy at {} Hz", freq1);
    assert!(spectrum[bin2] > 0.1, "Expected energy at {} Hz", freq2);
}

#[test]
fn test_spectrum_bands() {
    let samples = generate_white_noise(SAMPLE_RATE, 1.0, 1.0, 42);

    let mut analyzer = SpectrumAnalyzer::new(2048);
    let bands = analyzer.analyze_bands(&samples, SAMPLE_RATE, 32);

    assert_eq!(bands.len(), 32);

    // White noise should have relatively even energy across bands
    let avg: f32 = bands.iter().sum::<f32>() / bands.len() as f32;
    let variance: f32 = bands.iter().map(|&b| (b - avg).powi(2)).sum::<f32>() / bands.len() as f32;

    // Variance should be relatively low for white noise
    assert!(
        variance < 0.2,
        "White noise should have relatively even spectrum, variance = {}",
        variance
    );
}

#[test]
fn test_beat_detection_click_track() {
    let bpm = 120.0;
    let duration = 5.0;
    let samples = generate_click_track(bpm, SAMPLE_RATE, duration, 100.0);

    let beats = detect_beats(&samples, SAMPLE_RATE, 0.3);

    // Should detect approximately the right number of beats
    let expected_beats = (duration * bpm / 60.0) as usize;
    let detected = beats.len();

    assert!(
        detected >= expected_beats / 2,
        "Expected at least {} beats for {} BPM over {}s, got {}",
        expected_beats / 2,
        bpm,
        duration,
        detected
    );
}

#[test]
fn test_bpm_estimation() {
    let target_bpm = 140.0;
    let duration = 10.0;
    let samples = generate_click_track(target_bpm, SAMPLE_RATE, duration, 100.0);

    let beats = detect_beats(&samples, SAMPLE_RATE, 0.3);
    let estimated_bpm = estimate_bpm(&beats);

    // BPM should be within 10% of target
    let tolerance = target_bpm * 0.1;
    assert!(
        (estimated_bpm - target_bpm).abs() < tolerance,
        "Expected BPM ~{}, got {}",
        target_bpm,
        estimated_bpm
    );
}

#[test]
fn test_full_audio_analysis() {
    let bpm = 120.0;
    let duration = 4.0;
    let frame_rate = 30.0;
    let num_bands = 32;

    let samples = generate_test_beat(bpm, SAMPLE_RATE, duration);

    let analysis = analyze_audio(&samples, SAMPLE_RATE, frame_rate, num_bands);

    // Check basic properties
    assert!((analysis.duration - duration as f64).abs() < 0.1);
    assert_eq!(analysis.sample_rate, SAMPLE_RATE);
    assert_eq!(analysis.num_bands, num_bands);
    assert_eq!(analysis.frame_rate, frame_rate);

    // Should have approximately the right number of frames
    let expected_frames = (duration * frame_rate) as usize;
    assert!(
        (analysis.rms.len() as i32 - expected_frames as i32).abs() <= 2,
        "Expected ~{} frames, got {}",
        expected_frames,
        analysis.rms.len()
    );

    // Spectrum should have same number of entries as RMS
    assert_eq!(analysis.spectrum.len(), analysis.rms.len());

    // Each spectrum frame should have the right number of bands
    for frame in &analysis.spectrum {
        assert_eq!(frame.len(), num_bands);
    }

    // BPM should be estimated
    assert!(analysis.bpm > 0.0, "BPM should be positive");
}

#[test]
fn test_spectrum_analyzer_db() {
    let samples = generate_sine(1000.0, SAMPLE_RATE, 1.0, 0.5);

    let mut analyzer = SpectrumAnalyzer::new(2048);
    let db_spectrum = analyzer.analyze_db(&samples);

    // dB values should be in reasonable range
    for &db in &db_spectrum {
        assert!(
            (-80.0..=20.0).contains(&db),
            "dB value {} out of range",
            db
        );
    }

    // Peak should be higher than average
    let peak = db_spectrum
        .iter()
        .cloned()
        .fold(f32::NEG_INFINITY, f32::max);
    let avg: f32 = db_spectrum.iter().sum::<f32>() / db_spectrum.len() as f32;
    assert!(
        peak > avg,
        "Peak ({}) should be above average ({})",
        peak,
        avg
    );
}

#[test]
fn test_short_audio_analysis() {
    // Test with very short audio (less than 1 second)
    let samples = generate_sine(440.0, SAMPLE_RATE, 0.5, 1.0);

    let analysis = analyze_audio(&samples, SAMPLE_RATE, 30.0, 16);

    assert!(analysis.duration > 0.0);
    assert!(!analysis.rms.is_empty());
    assert!(!analysis.spectrum.is_empty());
}

#[test]
fn test_silence_analysis() {
    // Test with silence
    let samples = vec![0.0; SAMPLE_RATE as usize];

    let analysis = analyze_audio(&samples, SAMPLE_RATE, 30.0, 32);

    // RMS should be very low
    let max_rms = analysis.rms.iter().cloned().fold(0.0f32, f32::max);
    assert!(max_rms < 0.001, "Silence should have near-zero RMS");

    // Should detect no beats
    assert!(analysis.beats.is_empty(), "Silence should have no beats");
}
