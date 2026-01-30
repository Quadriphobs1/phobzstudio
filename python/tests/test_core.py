"""Tests for the phobz-viz core functionality."""

import pytest


@pytest.fixture
def core():
    """Get the Rust core module if available."""
    try:
        import phobz_visualizer

        return phobz_visualizer
    except ImportError:
        pytest.skip("Rust core not built. Run 'just build' first.")


def test_parse_color(core):
    """Test hex color parsing."""
    r, g, b = core.parse_color("#ff0000")
    assert r == pytest.approx(1.0)
    assert g == pytest.approx(0.0)
    assert b == pytest.approx(0.0)

    r, g, b = core.parse_color("#00ff00")
    assert r == pytest.approx(0.0)
    assert g == pytest.approx(1.0)
    assert b == pytest.approx(0.0)

    r, g, b = core.parse_color("0000ff")
    assert r == pytest.approx(0.0)
    assert g == pytest.approx(0.0)
    assert b == pytest.approx(1.0)


def test_parse_color_invalid(core):
    """Test invalid color parsing."""
    with pytest.raises(ValueError):
        core.parse_color("invalid")

    with pytest.raises(ValueError):
        core.parse_color("#fff")  # Too short


def test_platforms():
    """Test platform presets."""
    from phobz_viz.platforms import SHORTS, TIKTOK, YOUTUBE, Platform

    # Test preset values
    assert YOUTUBE.width == 1920
    assert YOUTUBE.height == 1080
    assert YOUTUBE.fps == 30

    assert TIKTOK.width == 1080
    assert TIKTOK.height == 1920
    assert TIKTOK.is_vertical

    assert SHORTS.width == 1080
    assert SHORTS.height == 1920
    assert SHORTS.is_vertical

    # Test from_name
    preset = Platform.from_name("youtube")
    assert preset.width == 1920

    preset = Platform.from_name("tiktok")
    assert preset.is_vertical

    # Test list_all
    all_presets = Platform.list_all()
    assert len(all_presets) >= 7

    # Test aspect ratio
    assert YOUTUBE.aspect_ratio == "16:9"
    assert TIKTOK.aspect_ratio == "9:16"


def test_cli_platforms():
    """Test CLI platforms command."""
    from typer.testing import CliRunner

    from phobz_viz.cli import app

    runner = CliRunner()
    result = runner.invoke(app, ["platforms"])

    assert result.exit_code == 0
    assert "youtube" in result.stdout.lower()
    assert "tiktok" in result.stdout.lower()


def test_cli_version():
    """Test CLI version command."""
    from typer.testing import CliRunner

    from phobz_viz.cli import app

    runner = CliRunner()
    result = runner.invoke(app, ["version"])

    assert result.exit_code == 0
    assert "0.1.0" in result.stdout
