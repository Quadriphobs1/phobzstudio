# Architecture

## Overview

Audio Visualiser is a GPU-accelerated video generation pipeline that transforms audio files into animated waveform visualizations.

```text
┌─────────────┐    ┌──────────────┐    ┌─────────────┐    ┌──────────────┐
│  Audio File │───▶│ Audio Module │───▶│ GPU Render  │───▶│ Video Encode │
│  (MP3/WAV)  │    │  (Analysis)  │    │   (wgpu)    │    │   (FFmpeg)   │
└─────────────┘    └──────────────┘    └─────────────┘    └──────────────┘
```

## Core Modules

### Audio (`core/src/audio/`)

Handles audio file loading and analysis.

| File | Purpose |
|------|---------|

| `loader.rs` | Load audio files via Symphonia (WAV, MP3, FLAC, AAC) |
| `fft.rs` | FFT spectrum analysis with Hann windowing |
| `analysis.rs` | Beat detection, BPM estimation, RMS energy |
| `synth.rs` | Synthetic test audio generation |

**Key Types:**

- `AudioData` - Loaded audio samples with metadata
- `SpectrumAnalyzer` - Real-time FFT analysis
- `AudioAnalysis` - Beat times, BPM, energy envelope

### GPU (`core/src/gpu/`)

Headless GPU rendering using wgpu.

| File | Purpose |
|------|---------|

| `context.rs` | GPU device/queue initialization |
| `pipeline.rs` | Render pipeline and shader management |
| `renderer.rs` | Frame rendering and pixel readback |
| `shaders/waveform.wgsl` | WGSL vertex/fragment shaders |

**Rendering Pipeline:**

1. Create GPU context (Metal on macOS, Vulkan on Linux)
2. Initialize render pipeline with waveform shader
3. For each frame:
   - Update uniform buffer (dimensions, colors, beat intensity)
   - Update instance buffer (bar heights)
   - Render to texture
   - Read back RGBA pixels

### Video (`core/src/video/`)

Video encoding via FFmpeg.

| File | Purpose |
|------|---------|

| `encoder.rs` | FFmpeg encoding wrapper |

**Supported Codecs:**

- H.264 (`libx264`) - Social media distribution
- ProRes 4444 (`prores_ks`) - Professional editing with transparency
- VP9 (`libvpx-vp9`) - Web use with transparency

## Data Flow

```text
1. Load Audio
   MP3/WAV ──▶ Symphonia ──▶ AudioData (f32 samples)

2. Analyze
   AudioData ──▶ FFT ──▶ Spectrum per frame
             ──▶ Beat Detection ──▶ Beat times
             ──▶ BPM Estimation ──▶ Tempo

3. Render Frame
   Spectrum + Beat ──▶ WaveformRenderer ──▶ RGBA pixels

4. Encode
   RGBA pixels ──▶ VideoEncoder ──▶ H.264/ProRes/VP9
```

## Technology Stack

| Layer | Technology | Reason |
|-------|------------|--------|

| Audio I/O | Symphonia | Pure Rust, multi-format |
| FFT | RustFFT | Fast, no dependencies |
| GPU | wgpu | Cross-platform, Metal/Vulkan |
| Video | rsmpeg (FFmpeg) | Industry standard codecs |
| Bindings | PyO3 | Python integration |

## Performance Considerations

### GPU Rendering

- Instanced rendering for bars (single draw call)
- Headless rendering to texture (no window required)
- Async buffer readback for pipelining

### Memory

- Audio loaded as f32 interleaved samples
- FFT uses in-place computation
- Frame buffers allocated once, reused per frame

### Encoding

- YUV color space conversion in Rust (avoids FFmpeg swscale)
- Direct buffer writes to FFmpeg frames
- Configurable CRF for quality/size tradeoff

## Future Considerations

- SIMD-accelerated color conversion
- Multi-threaded frame rendering
- GPU-based video encoding (NVENC/VideoToolbox)
