"""Platform presets for various social media platforms."""

from dataclasses import dataclass
from enum import Enum


@dataclass(frozen=True)
class PlatformPreset:
    """Configuration for a target platform."""

    name: str
    width: int
    height: int
    fps: int = 30
    max_duration_seconds: int | None = None

    @property
    def aspect_ratio(self) -> str:
        """Return aspect ratio as string (e.g., '16:9')."""
        from math import gcd

        divisor = gcd(self.width, self.height)
        w = self.width // divisor
        h = self.height // divisor
        return f"{w}:{h}"

    @property
    def is_vertical(self) -> bool:
        """Return True if this is a vertical (portrait) format."""
        return self.height > self.width


class Platform(Enum):
    """Available platform presets."""

    # YouTube
    YOUTUBE = PlatformPreset("youtube", 1920, 1080, 30)
    YOUTUBE_4K = PlatformPreset("youtube_4k", 3840, 2160, 30)
    YOUTUBE_SHORTS = PlatformPreset("shorts", 1080, 1920, 30, 60)

    # TikTok
    TIKTOK = PlatformPreset("tiktok", 1080, 1920, 30, 180)

    # Instagram
    INSTAGRAM_REELS = PlatformPreset("instagram_reels", 1080, 1920, 30, 90)
    INSTAGRAM_FEED = PlatformPreset("instagram", 1080, 1080, 30, 60)
    INSTAGRAM_PORTRAIT = PlatformPreset("instagram_portrait", 1080, 1350, 30, 60)

    @classmethod
    def from_name(cls, name: str) -> "PlatformPreset":
        """Get preset by name."""
        name_lower = name.lower().replace("-", "_")
        for platform in cls:
            if platform.value.name == name_lower:
                return platform.value
        raise ValueError(f"Unknown platform: {name}")

    @classmethod
    def list_all(cls) -> list["PlatformPreset"]:
        """List all available presets."""
        return [p.value for p in cls]


# Convenience aliases
YOUTUBE = Platform.YOUTUBE.value
YOUTUBE_4K = Platform.YOUTUBE_4K.value
SHORTS = Platform.YOUTUBE_SHORTS.value
TIKTOK = Platform.TIKTOK.value
INSTAGRAM = Platform.INSTAGRAM_FEED.value
INSTAGRAM_REELS = Platform.INSTAGRAM_REELS.value
INSTAGRAM_PORTRAIT = Platform.INSTAGRAM_PORTRAIT.value
