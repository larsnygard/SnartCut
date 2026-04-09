"""Shared enumerations and lightweight dataclasses used across all modules.

Keeping these in one place avoids circular imports between the canvas,
job, gcode and device modules.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Optional


# ---------------------------------------------------------------------------
# Enumerations
# ---------------------------------------------------------------------------


class DeviceType(Enum):
    """Supported machine types."""

    GRBL_LASER = "grbl_laser"
    GRBL_SPINDLE = "grbl_spindle"
    VINYL_CUTTER = "vinyl_cutter"
    MARLIN = "marlin"


class LayerMode(Enum):
    """How a cut layer is processed."""

    LINE = "line"          # Vector cut along the path
    FILL = "fill"          # Raster-fill the enclosed area
    OFFSET_FILL = "offset_fill"  # Concentric offset fill
    IMAGE = "image"        # Grayscale image engraving


class ToolType(Enum):
    """Canvas drawing tool identifiers."""

    SELECT = auto()
    PAN = auto()
    RECTANGLE = auto()
    ELLIPSE = auto()
    LINE = auto()
    POLYLINE = auto()
    BEZIER = auto()
    TEXT = auto()
    IMAGE = auto()
    MEASURE = auto()


class Units(Enum):
    """Linear measurement units shown in the UI."""

    MM = "mm"
    INCH = "inch"


class AirAssist(Enum):
    """Air-assist state for a laser pass."""

    OFF = "off"
    ON = "on"
    AUTO = "auto"


# ---------------------------------------------------------------------------
# Dataclasses
# ---------------------------------------------------------------------------


@dataclass
class Point:
    """2-D point in millimetres."""

    x: float = 0.0
    y: float = 0.0

    def __add__(self, other: "Point") -> "Point":
        return Point(self.x + other.x, self.y + other.y)

    def __sub__(self, other: "Point") -> "Point":
        return Point(self.x - other.x, self.y - other.y)

    def __repr__(self) -> str:
        return f"Point({self.x:.3f}, {self.y:.3f})"


@dataclass
class BoundingBox:
    """Axis-aligned bounding box in millimetres."""

    x: float = 0.0
    y: float = 0.0
    width: float = 0.0
    height: float = 0.0

    @property
    def right(self) -> float:
        return self.x + self.width

    @property
    def bottom(self) -> float:
        return self.y + self.height

    @property
    def center(self) -> Point:
        return Point(self.x + self.width / 2, self.y + self.height / 2)


@dataclass
class CutSettings:
    """Parameters for a single cut/engrave layer.

    Attributes:
        name:       Human-readable layer name.
        mode:       How the layer is processed (:class:`LayerMode`).
        speed_mm_s: Feed rate in mm/s.
        power_pct:  Laser power as a percentage 0–100.
        passes:     Number of repeated passes.
        z_offset_mm: Z-axis offset in mm (focus adjustment).
        air_assist: Air-assist state.
        enabled:    Whether the layer is included in the current job.
        color:      Display colour as ``#rrggbb`` hex string.
    """

    name: str = "Layer"
    mode: LayerMode = LayerMode.LINE
    speed_mm_s: float = 100.0
    power_pct: float = 50.0
    passes: int = 1
    z_offset_mm: float = 0.0
    air_assist: AirAssist = AirAssist.AUTO
    enabled: bool = True
    color: str = "#ff0000"

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "mode": self.mode.value,
            "speed_mm_s": self.speed_mm_s,
            "power_pct": self.power_pct,
            "passes": self.passes,
            "z_offset_mm": self.z_offset_mm,
            "air_assist": self.air_assist.value,
            "enabled": self.enabled,
            "color": self.color,
        }

    @classmethod
    def from_dict(cls, data: dict) -> "CutSettings":
        return cls(
            name=data.get("name", "Layer"),
            mode=LayerMode(data.get("mode", "line")),
            speed_mm_s=float(data.get("speed_mm_s", 100.0)),
            power_pct=float(data.get("power_pct", 50.0)),
            passes=int(data.get("passes", 1)),
            z_offset_mm=float(data.get("z_offset_mm", 0.0)),
            air_assist=AirAssist(data.get("air_assist", "auto")),
            enabled=bool(data.get("enabled", True)),
            color=data.get("color", "#ff0000"),
        )


@dataclass
class WorkspaceConfig:
    """Physical dimensions of the machine work area."""

    width_mm: float = 400.0
    height_mm: float = 400.0
    origin_bottom_left: bool = True  # True = Y increases upward
