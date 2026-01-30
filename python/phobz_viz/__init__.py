"""Phobz Visualizer - GPU-accelerated audio visualization for music producers.

This package provides tools for generating animated waveform videos synced to audio.

Usage:
    CLI:
        phobz-viz render beat.wav --platform youtube -o video.mp4
        phobz-viz analyze beat.wav -o analysis.json

    Python:
        from phobz_viz import render_video, analyze_audio

        analyze_audio("beat.wav", "analysis.json")
        render_video("beat.wav", "output.mp4", platform="youtube")
"""

__version__ = "0.1.0"

# Import from Rust core if available
try:
    from phobz_visualizer import analyze_audio as _analyze_audio
    from phobz_visualizer import parse_color
    from phobz_visualizer import render_video as _render_video

    def analyze_audio(audio_path: str) -> str:
        """Analyze audio file and return JSON analysis data.

        Args:
            audio_path: Path to audio file (WAV, MP3, FLAC, AAC)

        Returns:
            JSON string with analysis data including beats, BPM, duration
        """
        return _analyze_audio(audio_path)

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
        progress_callback=None,
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
            progress_callback: Optional callback function(float) for progress updates
        """
        return _render_video(
            audio_path,
            output_path,
            width=width,
            height=height,
            fps=fps,
            bar_count=bar_count,
            color=color,
            background=background,
            codec=codec,
            bitrate=bitrate,
            progress_callback=progress_callback,
        )

    __all__ = [
        "__version__",
        "analyze_audio",
        "render_video",
        "parse_color",
    ]

except ImportError:
    # Rust core not built yet
    __all__ = ["__version__"]
