#!/usr/bin/env python3
"""Generate synthetic test audio files using the Rust synth module.

Usage:
    python scripts/generate_test_audio.py

This creates test WAV files in the examples/ directory.
"""

from pathlib import Path


def main():
    try:
        import phobz_visualizer as core
    except ImportError:
        print("Error: Rust core not built. Run 'just build' first.")
        return 1

    output_dir = Path(__file__).parent.parent / "examples"
    output_dir.mkdir(exist_ok=True)

    print("Generating test audio files using Rust synth module...")
    print()

    # Generate a 5-second 120 BPM beat
    path = output_dir / "beat_120bpm.wav"
    print(f"Generating 120 BPM beat (5 seconds)...")
    core.generate_test_beat(str(path), bpm=120.0, duration=5.0)
    print(f"  Created: {path}")

    # Generate a 10-second 90 BPM beat
    path = output_dir / "beat_90bpm.wav"
    print(f"Generating 90 BPM beat (10 seconds)...")
    core.generate_test_beat(str(path), bpm=90.0, duration=10.0)
    print(f"  Created: {path}")

    # Generate a simple sine sweep (using click track as approximation)
    path = output_dir / "sine_sweep.wav"
    print(f"Generating sine wave (3 seconds)...")
    core.generate_sine(str(path), frequency=440.0, duration=3.0, amplitude=0.5)
    print(f"  Created: {path}")

    # Generate a click track
    path = output_dir / "click_track.wav"
    print(f"Generating click track (5 seconds)...")
    core.generate_click_track(str(path), bpm=120.0, duration=5.0)
    print(f"  Created: {path}")

    print()
    print("Done! Test audio files created in examples/")
    print()
    print("Test with:")
    print("  phobz-viz render examples/beat_120bpm.wav -o output.mp4")

    return 0


if __name__ == "__main__":
    exit(main())
