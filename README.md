# Audio visualisation

Audio waveform visualisation overlay generator

## Features

- **GPU-Accelerated FFT**: Compute shaders for real-time spectrum analysis (Cooley-Tukey radix-2)
- **Audio Analysis**: Load WAV, MP3, FLAC, AAC files with beat detection and BPM estimation
- **GPU Rendering**: Metal-accelerated waveform visualization on macOS (Vulkan on Linux)
- **Multiple Designs**: Bars, circular radial, circular ring, frame perimeter, and more
- **Post-Processing**: Glow effects with blur shaders
- **Video Export**: H.264 for social media, ProRes 4444 for transparent overlays, VP9 for web
- **Platform Presets**: Export for YouTube, TikTok, Instagram Reels, and more
- **Python CLI**: Simple command-line interface for rendering videos
- **CPU Fallback**: Automatic fallback to RustFFT when GPU is unavailable

## Requirements

- macOS with Apple Silicon (M1/M2/M3) or Intel with Metal support
- Nix package manager (for reproducible development environment)

## Quick Start

```bash
# Enter development environment
nix develop

# Build the project
just build

# Run tests
just test

# Render a video
phobz-viz render track.mp3 -o output.mp4

# With options
phobz-viz render track.mp3 -o output.mp4 \
  --platform tiktok \
  --design circular-ring \
  --bars 64 \
  --glow \
  --mirror \
  --color "#00ff88"
```

## Design Types

- `bars` - Traditional vertical bar visualization
- `circular-radial` - Radial bars emanating from center
- `circular-ring` - Ring of bars around a circle
- `frame-perimeter` - Bars along the frame edges

## Benchmarks

```bash
# Run all benchmarks
just bench

# Run specific benchmark group
cargo bench --bench audio_bench -- "GPU FFT"
cargo bench --bench audio_bench -- "CPU vs GPU"
```

## Development

See [docs/SETUP.md](docs/SETUP.md) for detailed setup instructions.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for technical architecture.

### Available Commands

```bash
just              # Show all available commands
just build        # Build Rust core and Python bindings
just test         # Run all tests
just test-rust    # Run Rust tests only
just check        # Type check without building
just fmt          # Format code
just lint         # Run linters
just clippy       # Run Clippy with all targets
just doc          # Build and open documentation
```

## License

TBD
