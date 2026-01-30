# Audio visualisation

Audio waveform visualisation overlay generator

## Features

- **Audio Analysis**: Load WAV, MP3, FLAC, AAC files with beat detection and BPM estimation
- **GPU Rendering**: Metal-accelerated waveform visualization on macOS (Vulkan on Linux)
- **Video Export**: H.264 for social media, ProRes 4444 for transparent overlays, VP9 for web
- **Platform Presets**: Export for YouTube, TikTok, Instagram Reels, and more
- **Python CLI**: CLI for rendering videos

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

# Render a video (after Python bindings are complete)
phobz-viz render track.mp3 --platform youtube -o output.mp4
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
