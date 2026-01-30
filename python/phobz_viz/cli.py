"""Phobz Visualizer CLI.

Command-line interface for generating audio visualizations.
"""

import json
from pathlib import Path
from typing import Annotated

import typer
from rich.console import Console
from rich.progress import BarColumn, Progress, SpinnerColumn, TextColumn
from rich.table import Table

from .platforms import Platform

app = typer.Typer(
    name="phobz-viz",
    help="GPU-accelerated audio visualization for music producers.",
    no_args_is_help=True,
)
console = Console()


def _get_core():
    """Import Rust core module."""
    try:
        import phobz_visualizer

        return phobz_visualizer
    except ImportError:
        console.print("[red]Error: Rust core not built. Run 'just build' first.[/red]")
        raise typer.Exit(1) from None


DESIGN_HELP = (
    "Visualization design (bars, circular-radial, circular-ring, frame-perimeter, "
    "frame-corners, waveform-line, spectrum-mountain, particles, spectrogram)"
)


@app.command()
def render(
    audio: Annotated[Path, typer.Argument(help="Path to audio file (WAV, MP3, FLAC)")],
    output: Annotated[Path, typer.Option("-o", "--output", help="Output video path")] = Path(
        "output.mp4"
    ),
    platform: Annotated[
        str, typer.Option("-p", "--platform", help="Platform preset (youtube, shorts, tiktok)")
    ] = "youtube",
    format: Annotated[
        str, typer.Option("-f", "--format", help="Output format (h264, prores4444, vp9)")
    ] = "h264",
    transparent: Annotated[
        bool, typer.Option("--transparent", help="Render with alpha channel (no background)")
    ] = False,
    color: Annotated[str, typer.Option("--color", help="Waveform color (hex)")] = "#00ff88",
    bars: Annotated[int, typer.Option("--bars", help="Number of waveform bars")] = 64,
    mirror: Annotated[
        bool, typer.Option("--mirror", help="Mirror waveform (symmetrical display)")
    ] = False,
    glow: Annotated[bool, typer.Option("--glow/--no-glow", help="Enable glow effect")] = True,
    design: Annotated[str, typer.Option("-d", "--design", help=DESIGN_HELP)] = "bars",
    verbose: Annotated[
        bool, typer.Option("-v", "--verbose", help="Show detailed processing info")
    ] = False,
) -> None:
    """Generate visualization video from audio file."""
    core = _get_core()

    # Validate audio file
    if not audio.exists():
        console.print(f"[red]Error: Audio file not found: {audio}[/red]")
        raise typer.Exit(1)

    # Get platform preset
    try:
        preset = Platform.from_name(platform)
    except ValueError:
        console.print(
            f"[red]Error: Unknown platform '{platform}'. "
            "Use 'phobz-viz platforms' to list available presets.[/red]"
        )
        raise typer.Exit(1) from None

    console.print("[bold green]Phobz Visualizer[/bold green]")
    console.print(f"Audio: {audio}")
    console.print(f"Output: {output}")
    console.print(f"Platform: {platform} ({preset.width}x{preset.height})")
    console.print(f"Format: {format}")
    console.print(f"Color: {color}")
    console.print(f"Bars: {bars}")
    console.print(f"Mirror: {mirror}")
    console.print(f"Glow: {glow}")
    console.print(f"Design: {design}")
    if verbose:
        console.print()
        console.print("[bold cyan]Processing Info:[/bold cyan]")
        console.print("  GPU FFT: [green]enabled[/green] (compute shaders)")
        console.print("  GPU Render: [green]enabled[/green] (Metal/Vulkan)")
        console.print(f"  FFT Size: 2048")
        console.print(f"  FPS: {preset.fps}")
    console.print()

    # Background color
    background = "#00000000" if transparent else "#000000"

    # Progress bar - disable stdout/stderr redirect in verbose mode to allow Rust logs
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        BarColumn(),
        TextColumn("[progress.percentage]{task.percentage:>3.0f}%"),
        console=console,
        redirect_stdout=not verbose,
        redirect_stderr=not verbose,
    ) as progress:
        task = progress.add_task("Rendering...", total=100)

        def update_progress(pct: float):
            progress.update(task, completed=int(pct * 100))

        try:
            core.render_video(
                str(audio),
                str(output),
                width=preset.width,
                height=preset.height,
                fps=preset.fps,
                bar_count=bars,
                color=color,
                background=background,
                codec=format,
                mirror=mirror,
                glow=glow,
                design=design,
                progress_callback=update_progress,
            )
        except Exception as e:
            console.print(f"[red]Error: {e}[/red]")
            raise typer.Exit(1) from None

    console.print(f"[bold green]Done![/bold green] Output: {output}")


@app.command()
def analyze(
    audio: Annotated[Path, typer.Argument(help="Path to audio file")],
    output: Annotated[Path, typer.Option("-o", "--output", help="Output JSON path")] = Path(
        "analysis.json"
    ),
) -> None:
    """Analyze audio and export data as JSON."""
    core = _get_core()

    if not audio.exists():
        console.print(f"[red]Error: Audio file not found: {audio}[/red]")
        raise typer.Exit(1)

    console.print(f"[bold green]Analyzing: {audio}[/bold green]")

    try:
        analysis_json = core.analyze_audio(str(audio))
        output.write_text(analysis_json)
    except Exception as e:
        console.print(f"[red]Error: {e}[/red]")
        raise typer.Exit(1) from None

    # Parse and display summary
    analysis = json.loads(analysis_json)
    console.print(f"Duration: {analysis.get('duration', 0):.2f}s")
    console.print(f"BPM: {analysis.get('bpm', 0):.1f}")
    console.print(f"Beats detected: {len(analysis.get('beats', []))}")
    console.print(f"Output: {output}")


@app.command()
def platforms() -> None:
    """List available platform presets."""
    table = Table(title="Platform Presets")
    table.add_column("Name", style="cyan")
    table.add_column("Resolution", style="green")
    table.add_column("Aspect", style="yellow")
    table.add_column("FPS")
    table.add_column("Max Duration")

    for preset in Platform.list_all():
        max_dur = f"{preset.max_duration_seconds}s" if preset.max_duration_seconds else "None"
        table.add_row(
            preset.name,
            f"{preset.width}x{preset.height}",
            preset.aspect_ratio,
            str(preset.fps),
            max_dur,
        )

    console.print(table)


@app.command()
def designs() -> None:
    """List available visualization designs."""
    core = _get_core()

    table = Table(title="Visualization Designs")
    table.add_column("Name", style="cyan")
    table.add_column("Description", style="green")

    for name, description in core.list_designs():
        table.add_row(name, description)

    console.print(table)
    console.print("\n[dim]Use --design <name> with the render command.[/dim]")


@app.command()
def version() -> None:
    """Show version information."""
    from phobz_viz import __version__

    console.print(f"[bold]phobz-viz[/bold] version {__version__}")

    try:
        core = _get_core()
        console.print(f"[bold]phobz-visualizer (core)[/bold] version {core.__version__}")
    except SystemExit:
        pass


if __name__ == "__main__":
    app()
