"""Top-level job settings and built-in material library.

:class:`JobSettings` holds the complete description of a cutting job:
workspace dimensions, material, and the :class:`~snartlaser.job.layer.LayerList`.

The :data:`MATERIAL_LIBRARY` dictionary provides sensible cut-setting presets
for common materials so users can get started quickly.
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Dict, Optional

from snartlaser.core.types import AirAssist, CutSettings, LayerMode, WorkspaceConfig
from snartlaser.job.layer import Layer, LayerList


# ---------------------------------------------------------------------------
# Built-in material presets
# ---------------------------------------------------------------------------

MATERIAL_LIBRARY: Dict[str, CutSettings] = {
    "Plywood 3mm (cut)": CutSettings(
        name="Plywood 3mm cut",
        mode=LayerMode.LINE,
        speed_mm_s=30.0,
        power_pct=90.0,
        passes=3,
        air_assist=AirAssist.ON,
        color="#e74c3c",
    ),
    "Plywood 3mm (engrave)": CutSettings(
        name="Plywood 3mm engrave",
        mode=LayerMode.FILL,
        speed_mm_s=200.0,
        power_pct=60.0,
        passes=1,
        air_assist=AirAssist.AUTO,
        color="#3498db",
    ),
    "Acrylic 3mm (cut)": CutSettings(
        name="Acrylic 3mm cut",
        mode=LayerMode.LINE,
        speed_mm_s=15.0,
        power_pct=95.0,
        passes=2,
        air_assist=AirAssist.ON,
        color="#9b59b6",
    ),
    "Cardboard (cut)": CutSettings(
        name="Cardboard cut",
        mode=LayerMode.LINE,
        speed_mm_s=80.0,
        power_pct=40.0,
        passes=1,
        air_assist=AirAssist.AUTO,
        color="#e67e22",
    ),
    "Leather (engrave)": CutSettings(
        name="Leather engrave",
        mode=LayerMode.FILL,
        speed_mm_s=150.0,
        power_pct=35.0,
        passes=1,
        air_assist=AirAssist.OFF,
        color="#795548",
    ),
    "Vinyl (cut)": CutSettings(
        name="Vinyl cut",
        mode=LayerMode.LINE,
        speed_mm_s=50.0,
        power_pct=0.0,   # blade force controlled separately
        passes=1,
        air_assist=AirAssist.OFF,
        color="#000000",
    ),
}


class JobSettings:
    """Complete description of a cutting/engraving job.

    Attributes:
        workspace:  Physical work area configuration.
        layers:     The ordered :class:`~snartlaser.job.layer.LayerList`.
        material:   Human-readable material name (informational).
        notes:      Free-form operator notes.
    """

    def __init__(self) -> None:
        self.workspace = WorkspaceConfig()
        self.layers = LayerList()
        self.material: str = ""
        self.notes: str = ""

    # ------------------------------------------------------------------
    # Convenience
    # ------------------------------------------------------------------

    def apply_preset(self, preset_name: str) -> Optional[Layer]:
        """Add a new layer pre-filled with the named *preset_name* settings.

        Returns the new :class:`~snartlaser.job.layer.Layer`, or ``None`` if
        the preset is not found.
        """
        preset = MATERIAL_LIBRARY.get(preset_name)
        if preset is None:
            return None
        import copy
        settings = copy.deepcopy(preset)
        layer = Layer(settings)
        self.layers._layers.append(layer)
        return layer

    # ------------------------------------------------------------------
    # Serialisation
    # ------------------------------------------------------------------

    def to_dict(self) -> dict:
        return {
            "workspace": {
                "width_mm": self.workspace.width_mm,
                "height_mm": self.workspace.height_mm,
                "origin_bottom_left": self.workspace.origin_bottom_left,
            },
            "layers": self.layers.to_dict(),
            "material": self.material,
            "notes": self.notes,
        }

    @classmethod
    def from_dict(cls, data: dict) -> "JobSettings":
        js = cls()
        ws = data.get("workspace", {})
        js.workspace.width_mm = float(ws.get("width_mm", 400.0))
        js.workspace.height_mm = float(ws.get("height_mm", 400.0))
        js.workspace.origin_bottom_left = bool(ws.get("origin_bottom_left", True))
        js.layers = LayerList.from_dict(data.get("layers", []))
        js.material = data.get("material", "")
        js.notes = data.get("notes", "")
        return js

    def save(self, path: str | Path) -> None:
        """Persist job settings as JSON to *path*."""
        Path(path).write_text(json.dumps(self.to_dict(), indent=2))

    @classmethod
    def load(cls, path: str | Path) -> "JobSettings":
        """Load job settings from a JSON file at *path*."""
        data = json.loads(Path(path).read_text())
        return cls.from_dict(data)
