//! Audio file loading using Symphonia.
//!
//! Supports WAV, MP3, FLAC, and AAC formats.

use std::fs::File;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use thiserror::Error;

/// Errors that can occur during audio loading.
#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Failed to open audio file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to probe audio format: {0}")]
    ProbeError(#[from] symphonia::core::errors::Error),

    #[error("No audio track found in file")]
    NoAudioTrack,

    #[error("Unknown sample rate")]
    UnknownSampleRate,
}

/// Audio data loaded from a file.
#[derive(Debug, Clone)]
pub struct AudioData {
    /// Interleaved audio samples (f32, normalized to -1.0..1.0)
    pub samples: Vec<f32>,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: usize,
}

impl AudioData {
    /// Duration of the audio in seconds.
    pub fn duration(&self) -> f64 {
        if self.sample_rate == 0 || self.channels == 0 {
            return 0.0;
        }
        self.samples.len() as f64 / (self.sample_rate as f64 * self.channels as f64)
    }

    /// Number of frames (samples per channel).
    pub fn num_frames(&self) -> usize {
        if self.channels == 0 {
            return 0;
        }
        self.samples.len() / self.channels
    }

    /// Convert stereo to mono by averaging channels.
    pub fn to_mono(&self) -> Vec<f32> {
        if self.channels == 1 {
            return self.samples.clone();
        }

        self.samples
            .chunks(self.channels)
            .map(|frame| frame.iter().sum::<f32>() / self.channels as f32)
            .collect()
    }
}

/// Load audio from a file path.
///
/// Supports WAV, MP3, FLAC, and AAC formats. The audio is decoded to
/// interleaved f32 samples normalized to the range -1.0..1.0.
///
/// # Example
///
/// ```no_run
/// use phobz_visualizer::audio::loader::load_audio;
/// use std::path::Path;
///
/// let audio = load_audio(Path::new("song.mp3")).unwrap();
/// println!("Duration: {:.2}s", audio.duration());
/// println!("Sample rate: {}Hz", audio.sample_rate);
/// println!("Channels: {}", audio.channels);
/// ```
pub fn load_audio(path: &Path) -> Result<AudioData, AudioError> {
    // Open the file
    let file = File::open(path)?;

    // Create a media source stream
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a hint to help with format detection
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    // Probe the format
    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;

    // Find the first audio track
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(AudioError::NoAudioTrack)?;

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or(AudioError::UnknownSampleRate)?;
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2);

    // Create decoder
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    // Decode all samples
    let mut samples = Vec::new();
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(symphonia::core::errors::Error::ResetRequired) => {
                // Reset decoder and continue
                decoder.reset();
                continue;
            }
            Err(e) => return Err(e.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => {
                // Skip decode errors
                continue;
            }
            Err(e) => return Err(e.into()),
        };

        // Initialize sample buffer on first decode
        if sample_buf.is_none() {
            let spec = *decoded.spec();
            let capacity = decoded.capacity() as u64;
            sample_buf = Some(SampleBuffer::new(capacity, spec));
        }

        if let Some(buf) = &mut sample_buf {
            buf.copy_interleaved_ref(decoded);
            samples.extend_from_slice(buf.samples());
        }
    }

    Ok(AudioData {
        samples,
        sample_rate,
        channels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_data_duration() {
        let audio = AudioData {
            samples: vec![0.0; 44100 * 2], // 1 second of stereo
            sample_rate: 44100,
            channels: 2,
        };
        assert!((audio.duration() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_audio_data_to_mono() {
        let audio = AudioData {
            samples: vec![0.5, -0.5, 1.0, 0.0], // 2 stereo frames
            sample_rate: 44100,
            channels: 2,
        };
        let mono = audio.to_mono();
        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 0.0).abs() < 0.001); // (0.5 + -0.5) / 2
        assert!((mono[1] - 0.5).abs() < 0.001); // (1.0 + 0.0) / 2
    }

    #[test]
    fn test_audio_data_num_frames() {
        let audio = AudioData {
            samples: vec![0.0; 44100 * 2],
            sample_rate: 44100,
            channels: 2,
        };
        assert_eq!(audio.num_frames(), 44100);
    }
}
