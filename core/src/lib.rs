//! Phobz Visualizer Core
//!
//! GPU-accelerated audio visualization library for generating animated waveform videos.
//!
//! # Features
//!
//! - Audio loading (WAV, MP3, FLAC, AAC) via Symphonia
//! - FFT spectrum analysis via RustFFT
//! - Beat detection and BPM estimation
//! - GPU rendering via wgpu (Metal on macOS, Vulkan on Linux)
//! - Video encoding via FFmpeg (H.264, ProRes 4444, VP9)
//! - Python bindings via PyO3 (when `python` feature is enabled)

pub mod audio;
pub mod designs;
pub mod gpu;
pub mod pipeline;
pub mod video;

// Re-export commonly used types
pub use audio::{analyze_audio, load_audio, AudioAnalysis, AudioData, SpectrumAnalyzer};
pub use designs::{
    create_design, default_params, BarsParams, CircularRadialParams, CircularRingParams, Design,
    DesignConfig, DesignParams, DesignType, Vertex,
};
pub use gpu::{DesignRenderConfig, DesignRenderer, GpuContext, RenderConfig, WaveformRenderer};
pub use pipeline::{
    analyze_audio_file, parse_hex_color, render_video, PipelineConfig, PipelineError,
};
pub use video::{VideoCodec, VideoConfig, VideoEncoder};

// Python bindings (only when python feature is enabled)
#[cfg(feature = "python")]
#[allow(deprecated)] // PyO3 0.27 deprecations - APIs still functional
mod python_bindings {
    use crate::pipeline::{self, PipelineConfig};
    use crate::video::VideoCodec;
    use pyo3::exceptions::PyRuntimeError;
    use pyo3::prelude::*;
    use pyo3::types::PyAny;
    use std::sync::{Arc, Mutex};

    /// Analyze audio file and return JSON analysis data.
    #[pyfunction]
    #[pyo3(signature = (audio_path))]
    fn analyze_audio(audio_path: &str) -> PyResult<String> {
        let analysis = pipeline::analyze_audio_file(audio_path)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        serde_json::to_string_pretty(&analysis).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Render visualization video from audio file.
    #[pyfunction]
    #[pyo3(signature = (audio_path, output_path, width=1920, height=1080, fps=30, bar_count=64, color="#00ff88", background="#000000", codec="h264", bitrate=8000000, mirror=false, glow=true, design="bars", progress_callback=None))]
    fn render_video(
        py: Python<'_>,
        audio_path: &str,
        output_path: &str,
        width: u32,
        height: u32,
        fps: u32,
        bar_count: u32,
        color: &str,
        background: &str,
        codec: &str,
        bitrate: u64,
        mirror: bool,
        glow: bool,
        design: &str,
        progress_callback: Option<Py<PyAny>>,
    ) -> PyResult<()> {
        let color_rgb = pipeline::parse_hex_color(color)
            .ok_or_else(|| PyRuntimeError::new_err(format!("Invalid color: {}", color)))?;
        let bg_rgb = pipeline::parse_hex_color(background).ok_or_else(|| {
            PyRuntimeError::new_err(format!("Invalid background: {}", background))
        })?;

        let video_codec = match codec.to_lowercase().as_str() {
            "h264" | "mp4" => VideoCodec::H264,
            "prores" | "prores4444" => VideoCodec::ProRes4444,
            "vp9" | "webm" => VideoCodec::Vp9,
            _ => return Err(PyRuntimeError::new_err(format!("Unknown codec: {}", codec))),
        };

        let design_type = crate::designs::DesignType::from_str(design).ok_or_else(|| {
            PyRuntimeError::new_err(format!(
                "Unknown design: {}. Available: bars, circular-radial, circular-ring",
                design
            ))
        })?;

        let config = PipelineConfig {
            width,
            height,
            fps,
            bar_count,
            color: color_rgb,
            background: bg_rgb,
            codec: video_codec,
            bitrate,
            fft_size: 2048,
            mirror,
            glow,
            design_type,
        };

        // Create callback wrapper
        let callback: Option<Box<dyn Fn(f32) + Send>> = progress_callback.map(|cb| {
            let cb = Arc::new(Mutex::new(cb));
            Box::new(move |progress: f32| {
                Python::with_gil(|py| {
                    let cb = cb.lock().unwrap();
                    let _ = cb.call1(py, (progress,));
                });
            }) as Box<dyn Fn(f32) + Send>
        });

        // Run the async render in a blocking context
        let audio = audio_path.to_string();
        let output = output_path.to_string();

        let result = py.allow_threads(|| {
            pollster::block_on(async {
                pipeline::render_video(&audio, &output, config, callback).await
            })
        });

        result.map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Parse hex color string to RGB tuple.
    #[pyfunction]
    fn parse_color(hex: &str) -> PyResult<(f32, f32, f32)> {
        let rgb = pipeline::parse_hex_color(hex)
            .ok_or_else(|| PyRuntimeError::new_err(format!("Invalid color: {}", hex)))?;
        Ok((rgb[0], rgb[1], rgb[2]))
    }

    /// Generate a test beat pattern and save to WAV file.
    #[pyfunction]
    #[pyo3(signature = (output_path, bpm=120.0, duration=5.0, sample_rate=44100))]
    fn generate_test_beat(
        output_path: &str,
        bpm: f32,
        duration: f32,
        sample_rate: u32,
    ) -> PyResult<()> {
        use crate::audio::synth;

        let samples = synth::generate_test_beat(bpm, sample_rate, duration);
        write_wav(std::path::Path::new(output_path), &samples, sample_rate)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Generate a sine wave and save to WAV file.
    #[pyfunction]
    #[pyo3(signature = (output_path, frequency=440.0, duration=1.0, amplitude=0.8, sample_rate=44100))]
    fn generate_sine(
        output_path: &str,
        frequency: f32,
        duration: f32,
        amplitude: f32,
        sample_rate: u32,
    ) -> PyResult<()> {
        use crate::audio::synth;

        let samples = synth::generate_sine(frequency, sample_rate, duration, amplitude);
        write_wav(std::path::Path::new(output_path), &samples, sample_rate)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Generate a click track (metronome) and save to WAV file.
    #[pyfunction]
    #[pyo3(signature = (output_path, bpm=120.0, duration=5.0, click_freq=1000.0, sample_rate=44100))]
    fn generate_click_track(
        output_path: &str,
        bpm: f32,
        duration: f32,
        click_freq: f32,
        sample_rate: u32,
    ) -> PyResult<()> {
        use crate::audio::synth;

        let samples = synth::generate_click_track(bpm, sample_rate, duration, click_freq);
        write_wav(std::path::Path::new(output_path), &samples, sample_rate)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Write samples to a WAV file.
    fn write_wav(path: &std::path::Path, samples: &[f32], sample_rate: u32) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::{BufWriter, Write};

        let mut file = BufWriter::new(File::create(path)?);

        let num_samples = samples.len() as u32;
        let byte_rate = sample_rate * 2; // 16-bit mono
        let data_size = num_samples * 2;
        let file_size = 36 + data_size;

        // RIFF header
        file.write_all(b"RIFF")?;
        file.write_all(&file_size.to_le_bytes())?;
        file.write_all(b"WAVE")?;

        // fmt chunk
        file.write_all(b"fmt ")?;
        file.write_all(&16u32.to_le_bytes())?; // chunk size
        file.write_all(&1u16.to_le_bytes())?; // PCM format
        file.write_all(&1u16.to_le_bytes())?; // mono
        file.write_all(&sample_rate.to_le_bytes())?;
        file.write_all(&byte_rate.to_le_bytes())?;
        file.write_all(&2u16.to_le_bytes())?; // block align
        file.write_all(&16u16.to_le_bytes())?; // bits per sample

        // data chunk
        file.write_all(b"data")?;
        file.write_all(&data_size.to_le_bytes())?;

        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let int_sample = (clamped * 32767.0) as i16;
            file.write_all(&int_sample.to_le_bytes())?;
        }

        Ok(())
    }

    /// List all available design types.
    #[pyfunction]
    fn list_designs() -> Vec<(String, String)> {
        crate::designs::DesignType::all()
            .iter()
            .map(|d| (d.name().to_string(), d.description().to_string()))
            .collect()
    }

    /// Phobz Visualizer Python module
    #[pymodule]
    pub fn phobz_visualizer(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add("__version__", env!("CARGO_PKG_VERSION"))?;
        m.add_function(wrap_pyfunction!(analyze_audio, m)?)?;
        m.add_function(wrap_pyfunction!(render_video, m)?)?;
        m.add_function(wrap_pyfunction!(parse_color, m)?)?;
        m.add_function(wrap_pyfunction!(generate_test_beat, m)?)?;
        m.add_function(wrap_pyfunction!(generate_sine, m)?)?;
        m.add_function(wrap_pyfunction!(generate_click_track, m)?)?;
        m.add_function(wrap_pyfunction!(list_designs, m)?)?;
        Ok(())
    }
}

#[cfg(feature = "python")]
pub use python_bindings::*;
