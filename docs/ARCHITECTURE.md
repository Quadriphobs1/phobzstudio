# Architecture

## Overview

Audio Visualiser is a GPU-accelerated video generation pipeline that transforms audio files into animated waveform visualizations.

```text
┌─────────────┐    ┌──────────────┐    ┌─────────────┐    ┌──────────────┐
│  Audio File │───▶│ Audio Module │───▶│ GPU Render  │───▶│ Video Encode │
│  (MP3/WAV)  │    │  (GPU FFT)   │    │   (wgpu)    │    │   (FFmpeg)   │
└─────────────┘    └──────────────┘    └─────────────┘    └──────────────┘
                          │
                          ▼
                   ┌──────────────┐
                   │ GPU Compute  │
                   │  (wgsl FFT)  │
                   └──────────────┘
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

Headless GPU rendering and compute using wgpu.

| File | Purpose |
|------|---------|

| `context.rs` | GPU device/queue initialization |
| `pipeline.rs` | Render pipeline and shader management |
| `renderer.rs` | Frame rendering and pixel readback |
| `design_renderer.rs` | Multi-design GPU renderer with post-processing |
| `postprocess.rs` | Glow/blur post-processing effects |
| `shaders/waveform.wgsl` | WGSL vertex/fragment shaders |
| `shaders/fft.wgsl` | GPU compute shaders for FFT |

#### GPU Compute (`core/src/gpu/compute/`)

GPU-accelerated audio processing using compute shaders.

| File | Purpose |
|------|---------|

| `fft.rs` | GPU FFT analyzer (Cooley-Tukey radix-2 DIT) |
| `spectrum.rs` | Spectrum pipeline with band grouping |
| `buffers.rs` | GPU buffer management for compute |
| `params.rs` | Uniform parameter structs (WGSL-aligned) |
| `pipelines.rs` | Compute pipeline creation |

**GPU FFT Pipeline:**

1. Upload audio samples to GPU buffer
2. Apply Hann window function (compute shader)
3. Bit-reversal permutation (compute shader)
4. Butterfly operations for each FFT stage (compute shader × log2(N) passes)
5. Compute magnitude spectrum (compute shader)
6. Optional: Group into logarithmic frequency bands
7. Read back results to CPU

**Rendering Pipeline:**

1. Create GPU context (Metal on macOS, Vulkan on Linux)
2. Initialize render pipeline with waveform shader
3. For each frame:
   - Run GPU FFT on audio samples
   - Update uniform buffer (dimensions, colors, beat intensity)
   - Update instance buffer (bar heights from FFT)
   - Render to texture
   - Apply post-processing (glow/blur if enabled)
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
| FFT (CPU) | RustFFT | Fast fallback, no dependencies |
| FFT (GPU) | wgpu compute | Parallel processing, low latency |
| GPU Render | wgpu | Cross-platform, Metal/Vulkan |
| Shaders | WGSL | Native wgpu shader language |
| Video | rsmpeg (FFmpeg) | Industry standard codecs |
| Bindings | PyO3 | Python integration |

## Performance Considerations

### GPU FFT

- Cooley-Tukey radix-2 decimation-in-time algorithm
- Parallel butterfly operations across workgroups (256 threads)
- Ping-pong buffer pattern to avoid synchronization overhead
- Single staging buffer for CPU readback
- Supports up to 2048 frequency bands

### GPU Rendering

- Instanced rendering for bars (single draw call)
- Headless rendering to texture (no window required)
- Async buffer readback for pipelining
- Post-processing glow effect with separable convolution

### Memory

- Audio loaded as f32 interleaved samples
- GPU buffers allocated once, reused per analysis
- Frame buffers allocated once, reused per frame
- Staging buffers shared between magnitude and band outputs

### Encoding

- YUV color space conversion in Rust (avoids FFmpeg swscale)
- Direct buffer writes to FFmpeg frames
- Configurable CRF for quality/size tradeoff

## CPU vs GPU FFT

| Aspect       | CPU (RustFFT)         | GPU (wgpu compute)      |
| ------------ | --------------------- | ----------------------- |
| Latency      | Lower for small sizes | Lower for large sizes   |
| Throughput   | Single-threaded       | Massively parallel      |
| Memory       | In-place              | Requires staging buffer |
| Availability | Always available      | Requires GPU context    |

The system automatically falls back to CPU FFT if GPU is unavailable.

## Future Considerations

- SIMD-accelerated color conversion
- Multi-threaded frame rendering
- GPU-based video encoding (NVENC/VideoToolbox)
- ML-based beat detection
- Source separation for stems
