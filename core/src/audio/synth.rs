//! Synthetic audio generation for testing.
//!
//! Generates test signals like sine waves, white noise, and click tracks
//! for unit and integration tests.

use std::f32::consts::PI;

/// Generate a sine wave.
///
/// # Arguments
/// * `frequency` - Frequency in Hz
/// * `sample_rate` - Sample rate in Hz
/// * `duration` - Duration in seconds
/// * `amplitude` - Amplitude (0.0 to 1.0)
pub fn generate_sine(frequency: f32, sample_rate: u32, duration: f32, amplitude: f32) -> Vec<f32> {
    let num_samples = (duration * sample_rate as f32) as usize;
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            amplitude * (2.0 * PI * frequency * t).sin()
        })
        .collect()
}

/// Generate white noise.
///
/// Uses a simple linear congruential generator for reproducibility.
pub fn generate_white_noise(
    sample_rate: u32,
    duration: f32,
    amplitude: f32,
    seed: u64,
) -> Vec<f32> {
    let num_samples = (duration * sample_rate as f32) as usize;

    // Simple LCG for reproducible "random" noise
    let mut state = seed;
    let a: u64 = 6364136223846793005;
    let c: u64 = 1442695040888963407;

    (0..num_samples)
        .map(|_| {
            state = state.wrapping_mul(a).wrapping_add(c);
            let normalized = (state as f32 / u64::MAX as f32) * 2.0 - 1.0;
            amplitude * normalized
        })
        .collect()
}

/// Generate a click track (metronome).
///
/// Creates short clicks at regular intervals based on BPM.
pub fn generate_click_track(
    bpm: f32,
    sample_rate: u32,
    duration: f32,
    click_freq: f32,
) -> Vec<f32> {
    let num_samples = (duration * sample_rate as f32) as usize;
    let samples_per_beat = (60.0 / bpm * sample_rate as f32) as usize;
    let click_samples = (sample_rate as f32 * 0.01) as usize; // 10ms click

    let mut samples = vec![0.0; num_samples];

    let mut pos = 0;
    while pos < num_samples {
        // Add a click (decaying sine)
        for i in 0..click_samples.min(num_samples - pos) {
            let t = i as f32 / sample_rate as f32;
            let envelope = (1.0 - i as f32 / click_samples as f32).powi(2);
            samples[pos + i] = envelope * (2.0 * PI * click_freq * t).sin();
        }
        pos += samples_per_beat;
    }

    samples
}

/// Generate a bass drum hit.
///
/// Creates a punchy bass sound for beat detection testing.
pub fn generate_kick(sample_rate: u32) -> Vec<f32> {
    let duration = 0.15; // 150ms
    let num_samples = (duration * sample_rate as f32) as usize;

    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;

            // Pitch envelope: starts at 150 Hz, drops to 50 Hz
            let freq = 50.0 + 100.0 * (-t * 30.0).exp();

            // Amplitude envelope: fast attack, medium decay
            let amp = (-t * 15.0).exp();

            amp * (2.0 * PI * freq * t).sin()
        })
        .collect()
}

/// Generate a test beat pattern.
///
/// Creates a simple 4/4 beat with kicks and hi-hats for comprehensive testing.
pub fn generate_test_beat(bpm: f32, sample_rate: u32, duration: f32) -> Vec<f32> {
    let num_samples = (duration * sample_rate as f32) as usize;
    let samples_per_beat = (60.0 / bpm * sample_rate as f32) as usize;
    let samples_per_16th = samples_per_beat / 4;

    let kick = generate_kick(sample_rate);
    let hihat_samples = (sample_rate as f32 * 0.05) as usize; // 50ms

    let mut samples = vec![0.0; num_samples];

    // Pattern: Kick on 1, 3; Hi-hat on all 8ths
    let mut pos = 0;
    let mut step = 0;

    while pos < num_samples {
        // Kick on beats 1 and 3 (steps 0 and 8)
        if step % 8 == 0 || step % 8 == 4 {
            for (i, &sample) in kick.iter().enumerate() {
                if pos + i < num_samples {
                    samples[pos + i] += sample * 0.8;
                }
            }
        }

        // Hi-hat on all 8th notes
        if step % 2 == 0 {
            for i in 0..hihat_samples.min(num_samples - pos) {
                let t = i as f32 / sample_rate as f32;
                let amp = (-t * 50.0).exp() * 0.3;
                // High frequency noise burst
                let noise = ((pos + i) as f32 * 12345.67).sin();
                samples[pos + i] += amp * noise;
            }
        }

        pos += samples_per_16th;
        step += 1;
    }

    // Normalize
    let max_val = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    if max_val > 1.0 {
        for s in &mut samples {
            *s /= max_val;
        }
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sine() {
        let samples = generate_sine(440.0, 44100, 1.0, 0.5);
        assert_eq!(samples.len(), 44100);

        // Check amplitude
        let max = samples.iter().cloned().fold(0.0f32, f32::max);
        assert!((max - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_generate_white_noise() {
        let samples = generate_white_noise(44100, 1.0, 1.0, 12345);
        assert_eq!(samples.len(), 44100);

        // Should have both positive and negative values
        let has_positive = samples.iter().any(|&s| s > 0.0);
        let has_negative = samples.iter().any(|&s| s < 0.0);
        assert!(has_positive && has_negative);
    }

    #[test]
    fn test_generate_click_track() {
        let samples = generate_click_track(120.0, 44100, 2.0, 1000.0);
        let expected_samples = (2.0 * 44100.0) as usize;
        assert_eq!(samples.len(), expected_samples);
    }

    #[test]
    fn test_generate_kick() {
        let kick = generate_kick(44100);
        assert!(!kick.is_empty());

        // Should have peak in first 10% and decay after that
        let peak_region = &kick[0..kick.len() / 10];
        let late_region = &kick[kick.len() / 2..];

        let peak_max = peak_region.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let late_max = late_region.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

        assert!(peak_max > late_max, "Kick should decay over time");
    }

    #[test]
    fn test_generate_test_beat() {
        let samples = generate_test_beat(120.0, 44100, 2.0);
        assert!(!samples.is_empty());

        // Should be normalized
        let max = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max <= 1.0);
    }
}
