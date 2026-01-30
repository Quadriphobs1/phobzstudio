"""Type stubs for phobz_visualizer Rust extension module."""

from typing import Callable

__version__: str

def analyze_audio(audio_path: str) -> str:
    """Analyze audio file and return JSON analysis data.

    Args:
        audio_path: Path to audio file (WAV, MP3, FLAC, AAC)

    Returns:
        JSON string with analysis data including beats, BPM, duration
    """
    ...

def render_video(
    audio_path: str,
    output_path: str,
    *,
    width: int = 1920,
    height: int = 1080,
    fps: int = 30,
    bar_count: int = 64,
    color: str = "#00ff88",
    background: str = "#000000",
    codec: str = "h264",
    bitrate: int = 8_000_000,
    mirror: bool = False,
    glow: bool = True,
    progress_callback: Callable[[float], None] | None = None,
) -> None:
    """Render visualization video from audio file.

    Args:
        audio_path: Path to audio file
        output_path: Path to output video file
        width: Video width in pixels
        height: Video height in pixels
        fps: Frames per second
        bar_count: Number of waveform bars
        color: Waveform color as hex string (e.g., "#00ff88")
        background: Background color as hex string
        codec: Video codec ("h264", "prores4444", "vp9")
        bitrate: Video bitrate in bits per second
        mirror: Mirror waveform (symmetrical display)
        glow: Enable glow effect
        progress_callback: Optional callback function(float) for progress updates
    """
    ...

def parse_color(hex: str) -> tuple[float, float, float]:
    """Parse hex color string to RGB tuple.

    Args:
        hex: Hex color string (e.g., "#ff0000" or "ff0000")

    Returns:
        Tuple of (r, g, b) floats in range 0.0-1.0
    """
    ...

def generate_test_beat(
    output_path: str,
    *,
    bpm: float = 120.0,
    duration: float = 5.0,
    sample_rate: int = 44100,
) -> None:
    """Generate a test beat pattern and save to WAV file.

    Args:
        output_path: Path to output WAV file
        bpm: Beats per minute
        duration: Duration in seconds
        sample_rate: Sample rate in Hz
    """
    ...

def generate_sine(
    output_path: str,
    *,
    frequency: float = 440.0,
    duration: float = 1.0,
    amplitude: float = 0.8,
    sample_rate: int = 44100,
) -> None:
    """Generate a sine wave and save to WAV file.

    Args:
        output_path: Path to output WAV file
        frequency: Frequency in Hz
        duration: Duration in seconds
        amplitude: Amplitude (0.0 to 1.0)
        sample_rate: Sample rate in Hz
    """
    ...

def generate_click_track(
    output_path: str,
    *,
    bpm: float = 120.0,
    duration: float = 5.0,
    click_freq: float = 1000.0,
    sample_rate: int = 44100,
) -> None:
    """Generate a click track (metronome) and save to WAV file.

    Args:
        output_path: Path to output WAV file
        bpm: Beats per minute
        duration: Duration in seconds
        click_freq: Click frequency in Hz
        sample_rate: Sample rate in Hz
    """
    ...
