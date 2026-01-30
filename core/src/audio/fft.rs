//! FFT spectrum analysis using RustFFT.
//!
//! Provides real-time spectrum analysis for audio visualization.

use rustfft::{num_complex::Complex, FftPlanner};

/// Spectrum analyzer for audio data.
///
/// Uses FFT to convert time-domain audio samples to frequency-domain
/// magnitude spectrum suitable for visualization.
pub struct SpectrumAnalyzer {
    planner: FftPlanner<f32>,
    fft_size: usize,
    window: Vec<f32>,
}

impl SpectrumAnalyzer {
    /// Create a new spectrum analyzer with the given FFT size.
    ///
    /// Common FFT sizes: 512, 1024, 2048, 4096
    /// Larger sizes give better frequency resolution but worse time resolution.
    pub fn new(fft_size: usize) -> Self {
        assert!(fft_size.is_power_of_two(), "FFT size must be a power of 2");

        // Create Hann window for smooth FFT (reduces spectral leakage)
        let window: Vec<f32> = (0..fft_size)
            .map(|i| {
                let t = i as f32 / (fft_size - 1) as f32;
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * t).cos())
            })
            .collect();

        Self {
            planner: FftPlanner::new(),
            fft_size,
            window,
        }
    }

    /// FFT size being used.
    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    /// Number of frequency bins in the output (FFT size / 2).
    pub fn num_bins(&self) -> usize {
        self.fft_size / 2
    }

    /// Compute magnitude spectrum from audio samples.
    ///
    /// Returns magnitudes for frequencies from 0 to Nyquist (sample_rate / 2).
    /// The returned vector has length `fft_size / 2`.
    ///
    /// # Panics
    ///
    /// Panics if `samples.len() < fft_size`.
    pub fn analyze(&mut self, samples: &[f32]) -> Vec<f32> {
        assert!(
            samples.len() >= self.fft_size,
            "Not enough samples: need {} but got {}",
            self.fft_size,
            samples.len()
        );

        // Apply window and convert to complex
        let mut buffer: Vec<Complex<f32>> = samples[..self.fft_size]
            .iter()
            .zip(&self.window)
            .map(|(s, w)| Complex::new(s * w, 0.0))
            .collect();

        // Plan and execute FFT
        let fft = self.planner.plan_fft_forward(self.fft_size);
        fft.process(&mut buffer);

        // Return magnitudes (only positive frequencies)
        buffer[..self.fft_size / 2]
            .iter()
            .map(|c| c.norm() / (self.fft_size as f32).sqrt())
            .collect()
    }

    /// Get the frequency in Hz for a given bin index.
    pub fn bin_to_freq(&self, bin: usize, sample_rate: u32) -> f32 {
        bin as f32 * sample_rate as f32 / self.fft_size as f32
    }

    /// Get the bin index for a given frequency in Hz.
    pub fn freq_to_bin(&self, freq: f32, sample_rate: u32) -> usize {
        (freq * self.fft_size as f32 / sample_rate as f32).round() as usize
    }

    /// Compute spectrum in decibels (dB).
    ///
    /// Converts linear magnitude to logarithmic scale.
    /// Returns values typically in range -80 to 0 dB.
    pub fn analyze_db(&mut self, samples: &[f32]) -> Vec<f32> {
        self.analyze(samples)
            .iter()
            .map(|&mag| {
                // 20 * log10(mag), clamped to avoid -inf
                let db = 20.0 * mag.max(1e-10).log10();
                db.max(-80.0) // Clamp to -80 dB floor
            })
            .collect()
    }

    /// Compute spectrum grouped into bands for visualization.
    ///
    /// Groups frequency bins into `num_bands` logarithmically-spaced bands,
    /// which better matches human perception of frequency.
    ///
    /// Returns magnitudes for each band, normalized to 0.0..1.0 range.
    pub fn analyze_bands(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        num_bands: usize,
    ) -> Vec<f32> {
        let spectrum = self.analyze(samples);
        let num_bins = spectrum.len();

        // Logarithmically spaced band edges from 20 Hz to Nyquist
        let min_freq = 20.0f32;
        let max_freq = sample_rate as f32 / 2.0;
        let log_min = min_freq.ln();
        let log_max = max_freq.ln();

        let mut bands = Vec::with_capacity(num_bands);

        for i in 0..num_bands {
            // Calculate frequency range for this band
            let t0 = i as f32 / num_bands as f32;
            let t1 = (i + 1) as f32 / num_bands as f32;

            let freq_low = (log_min + t0 * (log_max - log_min)).exp();
            let freq_high = (log_min + t1 * (log_max - log_min)).exp();

            // Convert to bin indices
            let bin_low = self.freq_to_bin(freq_low, sample_rate).min(num_bins - 1);
            let bin_high = self.freq_to_bin(freq_high, sample_rate).min(num_bins);

            // Average magnitudes in this band
            if bin_high > bin_low {
                let sum: f32 = spectrum[bin_low..bin_high].iter().sum();
                bands.push(sum / (bin_high - bin_low) as f32);
            } else {
                bands.push(spectrum.get(bin_low).copied().unwrap_or(0.0));
            }
        }

        // Normalize to 0.0..1.0
        let max_val = bands.iter().cloned().fold(0.0f32, f32::max);
        if max_val > 0.0 {
            for band in &mut bands {
                *band /= max_val;
            }
        }

        bands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn generate_sine(freq: f32, sample_rate: u32, num_samples: usize) -> Vec<f32> {
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * PI * freq * t).sin()
            })
            .collect()
    }

    #[test]
    fn test_spectrum_analyzer_creation() {
        let analyzer = SpectrumAnalyzer::new(1024);
        assert_eq!(analyzer.fft_size(), 1024);
        assert_eq!(analyzer.num_bins(), 512);
    }

    #[test]
    fn test_sine_wave_spectrum() {
        let sample_rate = 44100;
        let freq = 440.0; // A4 note
        let samples = generate_sine(freq, sample_rate, 4096);

        let mut analyzer = SpectrumAnalyzer::new(2048);
        let spectrum = analyzer.analyze(&samples);

        // Find the peak bin
        let peak_bin = spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        // The peak should be near 440 Hz
        let peak_freq = analyzer.bin_to_freq(peak_bin, sample_rate);
        assert!(
            (peak_freq - freq).abs() < 50.0, // Within 50 Hz
            "Expected peak near {} Hz, got {} Hz",
            freq,
            peak_freq
        );
    }

    #[test]
    fn test_bin_freq_conversion() {
        let analyzer = SpectrumAnalyzer::new(2048);
        let sample_rate = 44100;

        // Nyquist should be at the last bin
        let nyquist = sample_rate as f32 / 2.0;
        let nyquist_bin = analyzer.freq_to_bin(nyquist, sample_rate);
        assert_eq!(nyquist_bin, 1024);

        // 1000 Hz
        let bin = analyzer.freq_to_bin(1000.0, sample_rate);
        let freq = analyzer.bin_to_freq(bin, sample_rate);
        assert!((freq - 1000.0).abs() < 50.0);
    }

    #[test]
    fn test_analyze_bands() {
        let sample_rate = 44100;
        let samples = generate_sine(1000.0, sample_rate, 4096);

        let mut analyzer = SpectrumAnalyzer::new(2048);
        let bands = analyzer.analyze_bands(&samples, sample_rate, 32);

        assert_eq!(bands.len(), 32);

        // All bands should be in range 0.0..=1.0
        for &band in &bands {
            assert!((0.0..=1.0).contains(&band));
        }
    }

    #[test]
    fn test_analyze_db() {
        let sample_rate = 44100;
        let samples = generate_sine(1000.0, sample_rate, 4096);

        let mut analyzer = SpectrumAnalyzer::new(2048);
        let db_spectrum = analyzer.analyze_db(&samples);

        // dB values should be in reasonable range
        for &db in &db_spectrum {
            assert!((-80.0..=20.0).contains(&db));
        }
    }
}
