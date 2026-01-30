//! Video encoding using FFmpeg via rsmpeg.
//!
//! Provides video encoding capabilities for:
//! - H.264 for YouTube/TikTok/Instagram
//! - ProRes 4444 for professional workflows with transparency
//! - WebM VP9 for web use

pub mod encoder;

pub use encoder::{VideoCodec, VideoConfig, VideoEncoder, VideoError};
