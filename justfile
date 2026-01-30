# Phobz Audio Visualizer - Task Runner
# Run `just` to see available commands

# Default recipe
default:
    @just --list

# Build Rust core and Python bindings
build:
    cd core && cargo build --release
    cd python && maturin develop --release
    @# Add Python source to path for editable install
    @echo "$(pwd)/python" > .venv/lib/python3.14/site-packages/phobz_viz.pth

# Build in debug mode
build-debug:
    cd core && cargo build
    cd python && maturin develop
    @# Add Python source to path for editable install
    @echo "$(pwd)/python" > .venv/lib/python3.14/site-packages/phobz_viz.pth

# Run all tests
test:
    cd core && cargo test
    pytest python/

# Run Rust tests only
test-rust:
    cd core && cargo test

# Run Rust tests with output
test-rust-verbose:
    cd core && cargo test -- --nocapture

# Run Python tests only
test-python:
    pytest python/

# Type check Rust code without building
check:
    cd core && cargo check

# Build and open Rust documentation
doc:
    cd core && cargo doc --open

# Run clippy with all targets
clippy:
    cd core && cargo clippy --all-targets -- -D warnings

# Auto-rebuild on file changes
dev:
    cargo watch -C core -x build -x test

# Format code
fmt:
    cd core && cargo fmt
    ruff format python/

# Lint code
lint:
    cd core && cargo clippy -- -D warnings
    ruff check python/

# Clean build artifacts
clean:
    cd core && cargo clean
    rm -rf .venv
    rm -rf python/*.egg-info
    find . -name "*.pyc" -delete
    find . -name "__pycache__" -delete

# === Visualization Commands ===

# Quick render to YouTube format
viz AUDIO OUTPUT="output.mp4":
    phobz-viz render {{AUDIO}} --platform youtube -o {{OUTPUT}}

# Render for YouTube (16:9 1080p)
youtube AUDIO OUTPUT="youtube.mp4":
    phobz-viz render {{AUDIO}} --platform youtube -o {{OUTPUT}}

# Render for YouTube 4K
youtube-4k AUDIO OUTPUT="youtube_4k.mp4":
    phobz-viz render {{AUDIO}} --platform youtube_4k -o {{OUTPUT}}

# Render for YouTube Shorts (9:16)
shorts AUDIO OUTPUT="shorts.mp4":
    phobz-viz render {{AUDIO}} --platform shorts -o {{OUTPUT}}

# Render for TikTok (9:16)
tiktok AUDIO OUTPUT="tiktok.mp4":
    phobz-viz render {{AUDIO}} --platform tiktok -o {{OUTPUT}}

# Render for Instagram Reels (9:16)
reels AUDIO OUTPUT="reels.mp4":
    phobz-viz render {{AUDIO}} --platform instagram_reels -o {{OUTPUT}}

# Render for Instagram Feed (1:1)
instagram AUDIO OUTPUT="instagram.mp4":
    phobz-viz render {{AUDIO}} --platform instagram -o {{OUTPUT}}

# Render for Instagram Portrait (4:5)
instagram-portrait AUDIO OUTPUT="instagram_portrait.mp4":
    phobz-viz render {{AUDIO}} --platform instagram_portrait -o {{OUTPUT}}

# Render all platform variants
all-platforms AUDIO:
    just youtube {{AUDIO}} youtube.mp4
    just shorts {{AUDIO}} shorts.mp4
    just tiktok {{AUDIO}} tiktok.mp4
    just instagram {{AUDIO}} instagram.mp4

# === Transparent Overlay Exports ===

# ProRes 4444 transparent overlay for DaVinci Resolve
overlay-prores AUDIO OUTPUT="overlay.mov":
    phobz-viz render {{AUDIO}} --format prores4444 --transparent -o {{OUTPUT}}

# WebM VP9 transparent overlay for Remotion/web
overlay-webm AUDIO OUTPUT="overlay.webm":
    phobz-viz render {{AUDIO}} --format webm --transparent -o {{OUTPUT}}

# PNG sequence export
png-sequence AUDIO OUTPUT_DIR="frames":
    phobz-viz render {{AUDIO}} --format png-sequence -o {{OUTPUT_DIR}}

# === Analysis Export ===

# Export analysis JSON for Remotion
export-json AUDIO OUTPUT="analysis.json":
    phobz-viz analyze {{AUDIO}} -o {{OUTPUT}}

# === Examples & Testing ===

# Generate test audio files
gen-test-audio:
    python scripts/generate_test_audio.py

# Run Rust example with synthetic audio
example-synthetic:
    cd core && cargo run --example render_synthetic --features tokio

# Run full test demo (generate audio + render)
demo: gen-test-audio
    @echo "Rendering demo video..."
    phobz-viz render examples/beat_120bpm.wav -o demo_output.mp4 --bars 32
    @echo "Done! Play with: ffplay demo_output.mp4"

# Run vertical layout demo
demo-vertical: gen-test-audio
    @echo "Rendering vertical demo for TikTok/Shorts..."
    phobz-viz render examples/beat_120bpm.wav -o demo_vertical.mp4 --platform shorts --bars 24
    @echo "Done! Play with: ffplay demo_vertical.mp4"

# === Development ===

# Show available platform presets
platforms:
    phobz-viz platforms

# Check FFmpeg ProRes support
check-prores:
    ffmpeg -encoders 2>&1 | grep prores

# Check FFmpeg version
check-ffmpeg:
    ffmpeg -version | head -5

# Run all benchmarks
bench:
    cd core && cargo bench

# Run audio benchmarks only
bench-audio:
    cd core && cargo bench --bench audio_bench

# Run render benchmarks only
bench-render:
    cd core && cargo bench --bench render_bench

# Verify all components work
verify:
    @echo "=== Verifying Phobz Visualizer ==="
    @echo ""
    @echo "1. Checking Rust build..."
    cd core && cargo check
    @echo "✓ Rust code compiles"
    @echo ""
    @echo "2. Running Rust tests..."
    cd core && cargo test --quiet
    @echo "✓ All Rust tests pass"
    @echo ""
    @echo "3. Checking Python imports..."
    python -c "from phobz_viz import __version__; print(f'✓ phobz_viz v{__version__} importable')"
    @echo ""
    @echo "=== All verifications passed! ==="
