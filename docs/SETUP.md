# Setup Guide

## Prerequisites

### 1. Install Nix

Nix provides a reproducible development environment with all required dependencies.

```bash
# macOS/Linux
curl -L https://nixos.org/nix/install | sh

# Enable flakes (add to ~/.config/nix/nix.conf)
experimental-features = nix-command flakes
```

### 2. Install direnv (Optional)

Direnv automatically loads the Nix environment when entering the project directory.

```bash
# macOS
brew install direnv

# Add to shell config (~/.zshrc or ~/.bashrc)
eval "$(direnv hook zsh)"  # or bash
```

## Getting Started

### Clone and Enter Environment

```bash
# With direnv (automatic)
direnv allow

# Without direnv (manual)
nix develop
```

### Verify Installation

```bash
# Check versions
rustc --version    # Should show 1.93+
python --version   # Should show 3.14+
ffmpeg -version    # Should show 8.0+
bun --version      # Should show 1.3+

# Run tests
just test
```

## Development Environment

The Nix flake provides:

| Tool | Version | Purpose |
|------|---------|---------|

| Rust | 1.93+ | Core library |
| Python | 3.14 | CLI and bindings |
| FFmpeg | 8.0 | Video encoding |
| Bun | 1.3 | JavaScript runtime |
| just | latest | Task runner |

### Environment Variables

The development shell sets:

- `PKG_CONFIG_PATH` - FFmpeg library discovery
- `DYLD_LIBRARY_PATH` - Dynamic library loading (macOS)
- `RUST_BACKTRACE=1` - Detailed error backtraces

## Building

### Rust Core

```bash
# Debug build
just build-debug

# Release build
just build

# Type check only
just check
```

### Python Bindings

```bash
# Development install
maturin develop --manifest-path core/Cargo.toml

# Release wheel
maturin build --manifest-path core/Cargo.toml --release
```

## Testing

### Running Tests

```bash
# All tests
just test

# Rust only
just test-rust

# Python only
just test-python

# With output
just test-rust-verbose
```

### Generating Test Audio

The project includes a script to generate synthetic test audio files for testing:

```bash
# Generate test audio files in examples/ directory
just gen-test-audio
```

This creates:

- `examples/beat_120bpm.wav` - 120 BPM kick drum pattern (4 seconds)
- `examples/beat_90bpm.wav` - 90 BPM kick drum pattern (4 seconds)
- `examples/sine_sweep.wav` - 20Hz to 8000Hz frequency sweep (4 seconds)

### Running Examples

#### Rust Example

The Rust example demonstrates the full render pipeline with synthetic audio:

```bash
# Run the Rust synthetic audio example
just example-synthetic
```

This generates:

- `test_output/synthetic_beat.wav` - Generated beat audio
- `test_output/synthetic_render.mp4` - Rendered visualization video

#### Demo Commands

```bash
# Run full demo (generates audio + renders video)
just demo

# Run vertical (9:16) demo for social media
just demo-vertical
```

### Verification

To verify all components are working correctly:

```bash
just verify
```

This runs:

1. Rust compilation check
2. Unit tests
3. Clippy lints
4. Format check
5. Example compilation

### Manual Testing

#### Test Audio Analysis

```python
from phobz_viz import analyze_audio
import json

# Analyze an audio file
result = analyze_audio("examples/beat_120bpm.wav")
data = json.loads(result)
print(f"Duration: {data['duration']:.2f}s")
print(f"BPM: {data['bpm']:.1f}")
print(f"Beats detected: {len(data['beats'])}")
```

#### Test Video Rendering

```python
from phobz_viz import render_video

# Render with progress callback
def on_progress(pct):
    print(f"Progress: {pct*100:.1f}%")

render_video(
    "examples/beat_120bpm.wav",
    "output.mp4",
    width=1920,
    height=1080,
    fps=30,
    bar_count=64,
    color="#00ff88",
    progress_callback=on_progress
)
```

#### Test CLI

```bash
# Analyze audio
phobz-viz analyze examples/beat_120bpm.wav -o analysis.json

# Render video
phobz-viz render examples/beat_120bpm.wav -o output.mp4

# Render for TikTok/Shorts (vertical)
phobz-viz render examples/beat_120bpm.wav -o vertical.mp4 --platform tiktok

# List available platforms
phobz-viz platforms
```

## Troubleshooting

### FFmpeg Not Found

Ensure you're in the Nix shell:

```bash
nix develop
# or
direnv allow
```

### Metal/GPU Issues

Check GPU availability:

```bash
# The test will skip gracefully if no GPU is available
cargo test gpu::context::tests::test_gpu_context_creation
```

### Build Cache Issues

Clean and rebuild:

```bash
just clean
just build
```
