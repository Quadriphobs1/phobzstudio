//! Audio analysis: RMS energy, beat detection, BPM estimation.
//!
//! Provides analysis functions for generating visualization data.

use super::fft::SpectrumAnalyzer;
use serde::{Deserialize, Serialize};

/// Information about a detected beat.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BeatInfo {
    /// Time of the beat in seconds.
    pub time: f64,
    /// Strength/confidence of the beat (0.0 to 1.0).
    pub strength: f32,
}

/// Complete analysis of an audio track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioAnalysis {
    /// Duration in seconds.
    pub duration: f64,
    /// Detected beats.
    pub beats: Vec<BeatInfo>,
    /// RMS energy per frame.
    pub rms: Vec<f32>,
    /// Spectrum data per frame.
    pub spectrum: Vec<Vec<f32>>,
    /// Number of spectrum bands.
    pub num_bands: usize,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Estimated BPM (beats per minute).
    pub bpm: f32,
    /// Frame rate (frames per second).
    pub frame_rate: f32,
}

/// Calculate RMS (Root Mean Square) energy of audio samples.
///
/// RMS is a measure of audio loudness that's more perceptually
/// accurate than peak levels.
pub fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// Detect beats using energy-based onset detection.
///
/// This uses a simple but effective approach:
/// 1. Compute energy in bass frequency range
/// 2. Compare to local average energy
/// 3. Detect peaks that exceed threshold
pub fn detect_beats(samples: &[f32], sample_rate: u32, sensitivity: f32) -> Vec<BeatInfo> {
    let fft_size = 1024;
    let hop_size = 512; // ~11ms at 44.1kHz

    // Not enough samples
    if samples.len() < fft_size {
        return Vec::new();
    }

    let mut analyzer = SpectrumAnalyzer::new(fft_size);
    let num_windows = (samples.len() - fft_size) / hop_size + 1;

    // Calculate bass energy for each window (20-200 Hz)
    let bass_low_bin = analyzer.freq_to_bin(20.0, sample_rate);
    let bass_high_bin = analyzer.freq_to_bin(200.0, sample_rate);

    let mut bass_energy: Vec<f32> = Vec::with_capacity(num_windows);

    for i in 0..num_windows {
        let start = i * hop_size;
        let spectrum = analyzer.analyze(&samples[start..start + fft_size]);

        // Sum energy in bass range
        let energy: f32 = spectrum[bass_low_bin..bass_high_bin.min(spectrum.len())]
            .iter()
            .map(|&m| m * m)
            .sum();
        bass_energy.push(energy);
    }

    // Normalize bass energy
    let max_energy = bass_energy.iter().cloned().fold(0.0f32, f32::max);
    if max_energy > 0.0 {
        for e in &mut bass_energy {
            *e /= max_energy;
        }
    }

    // Calculate local average using a sliding window
    let avg_window = 8; // ~100ms context window
    let mut local_avg: Vec<f32> = Vec::with_capacity(bass_energy.len());

    for i in 0..bass_energy.len() {
        let start = i.saturating_sub(avg_window);
        let end = (i + avg_window + 1).min(bass_energy.len());
        let avg: f32 = bass_energy[start..end].iter().sum::<f32>() / (end - start) as f32;
        local_avg.push(avg);
    }

    // Detect peaks that exceed local average by threshold
    let threshold = 1.0 + sensitivity; // sensitivity = 0.5 means 50% above average
    let min_beat_spacing = (sample_rate as f32 / hop_size as f32 * 0.2) as usize; // 200ms minimum

    let mut beats = Vec::new();
    let mut last_beat: Option<usize> = None;

    for i in 1..bass_energy.len() - 1 {
        // Check if this is a local peak
        let is_peak = bass_energy[i] > bass_energy[i - 1] && bass_energy[i] > bass_energy[i + 1];

        // Check if it exceeds threshold
        let exceeds_threshold = bass_energy[i] > local_avg[i] * threshold;

        // Check minimum spacing from last beat
        let enough_spacing = last_beat.map_or(true, |lb| i - lb >= min_beat_spacing);

        if is_peak && exceeds_threshold && enough_spacing {
            let time = (i * hop_size) as f64 / sample_rate as f64;
            let strength = (bass_energy[i] / local_avg[i].max(0.01) - 1.0).min(1.0);

            beats.push(BeatInfo { time, strength });
            last_beat = Some(i);
        }
    }

    beats
}

/// Estimate BPM from detected beats.
///
/// Uses average interval between beats to estimate tempo.
pub fn estimate_bpm(beats: &[BeatInfo]) -> f32 {
    if beats.len() < 2 {
        return 0.0;
    }

    // Calculate intervals between consecutive beats
    let intervals: Vec<f64> = beats
        .windows(2)
        .map(|w| w[1].time - w[0].time)
        .filter(|&i| i > 0.2 && i < 2.0) // Filter unrealistic intervals
        .collect();

    if intervals.is_empty() {
        return 0.0;
    }

    // Take median interval to be robust against outliers
    let mut sorted = intervals.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_interval = sorted[sorted.len() / 2];

    // Convert to BPM
    let bpm = 60.0 / median_interval;

    // Clamp to reasonable range
    if bpm < 60.0 {
        (bpm * 2.0) as f32 // Double if too slow
    } else if bpm > 200.0 {
        (bpm / 2.0) as f32 // Halve if too fast
    } else {
        bpm as f32
    }
}

/// Perform complete analysis of audio data.
///
/// Generates all data needed for visualization.
/// Expects mono audio samples. Use `AudioData::to_mono()` to convert stereo first.
pub fn analyze_audio(
    samples: &[f32],
    sample_rate: u32,
    frame_rate: f32,
    num_bands: usize,
) -> AudioAnalysis {
    let duration = samples.len() as f64 / sample_rate as f64;
    let beats = detect_beats(samples, sample_rate, 0.5);
    let bpm = estimate_bpm(&beats);

    let samples_per_frame = (sample_rate as f32 / frame_rate) as usize;
    let num_frames = (samples.len() / samples_per_frame).max(1);
    let fft_size = 2048;
    let mut analyzer = SpectrumAnalyzer::new(fft_size);

    let mut rms = Vec::with_capacity(num_frames);
    let mut spectrum = Vec::with_capacity(num_frames);

    for i in 0..num_frames {
        let start = i * samples_per_frame;
        let end = (start + samples_per_frame).min(samples.len());
        let frame_samples = &samples[start..end];

        // RMS for this frame
        rms.push(calculate_rms(frame_samples));

        // Spectrum bands for this frame
        if frame_samples.len() >= fft_size {
            let bands = analyzer.analyze_bands(frame_samples, sample_rate, num_bands);
            spectrum.push(bands);
        } else {
            let bands = analyzer.analyze_bands(
                &{
                    let mut padded = vec![0.0; fft_size];
                    padded[..frame_samples.len()].copy_from_slice(frame_samples);
                    padded
                },
                sample_rate,
                num_bands,
            );
            spectrum.push(bands);
        }
    }

    AudioAnalysis {
        duration,
        sample_rate,
        bpm,
        beats,
        rms,
        spectrum,
        num_bands,
        frame_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn generate_click_track(bpm: f32, sample_rate: u32, duration_sec: f32) -> Vec<f32> {
        let num_samples = (duration_sec * sample_rate as f32) as usize;
        let samples_per_beat = (60.0 / bpm * sample_rate as f32) as usize;
        let click_duration = 100; // samples

        let mut samples = vec![0.0; num_samples];

        let mut pos = 0;
        while pos < num_samples {
            // Add a click (short burst)
            for i in 0..click_duration.min(num_samples - pos) {
                let t = i as f32 / click_duration as f32;
                let envelope = (PI * t).sin(); // Simple envelope
                samples[pos + i] = envelope * (2.0 * PI * 100.0 * t).sin(); // 100 Hz click
            }
            pos += samples_per_beat;
        }

        samples
    }

    #[test]
    fn test_calculate_rms() {
        // RMS of a sine wave should be 1/sqrt(2) ≈ 0.707
        let samples: Vec<f32> = (0..1000)
            .map(|i| (2.0 * PI * i as f32 / 100.0).sin())
            .collect();
        let rms = calculate_rms(&samples);
        assert!((rms - 0.707).abs() < 0.01);
    }

    #[test]
    fn test_detect_beats_basic() {
        let sample_rate = 44100;
        let bpm = 120.0;
        let samples = generate_click_track(bpm, sample_rate, 5.0);

        let beats = detect_beats(&samples, sample_rate, 0.3);

        // Should detect some beats
        assert!(!beats.is_empty(), "Should detect beats in click track");

        // Beats should be roughly 0.5 seconds apart (120 BPM)
        if beats.len() >= 2 {
            let interval = beats[1].time - beats[0].time;
            assert!(
                (interval - 0.5).abs() < 0.1,
                "Beat interval should be ~0.5s for 120 BPM, got {}s",
                interval
            );
        }
    }

    #[test]
    fn test_estimate_bpm() {
        // Create beats at 120 BPM (0.5s intervals)
        let beats: Vec<BeatInfo> = (0..10)
            .map(|i| BeatInfo {
                time: i as f64 * 0.5,
                strength: 1.0,
            })
            .collect();

        let bpm = estimate_bpm(&beats);
        assert!((bpm - 120.0).abs() < 5.0, "Expected ~120 BPM, got {}", bpm);
    }

    #[test]
    fn test_analyze_audio() {
        let sample_rate = 44100;
        let samples: Vec<f32> =
            (0..sample_rate * 2) // 2 seconds
                .map(|i| (2.0 * PI * 440.0 * i as f32 / sample_rate as f32).sin())
                .collect();

        let analysis = analyze_audio(&samples, sample_rate, 30.0, 32);

        assert!((analysis.duration - 2.0).abs() < 0.1);
        assert_eq!(analysis.sample_rate, sample_rate);
        assert_eq!(analysis.num_bands, 32);
        assert!(!analysis.rms.is_empty());
        assert!(!analysis.spectrum.is_empty());
    }
}
