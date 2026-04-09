"""Persistent application configuration backed by :class:`QSettings`.

All settings are accessed through typed properties.  Keys follow the
``section/key`` naming convention used by :class:`QSettings`.

Example::

    cfg = Config()
    cfg.last_directory = "/home/user/designs"
    print(cfg.last_directory)
"""
from __future__ import annotations

from pathlib import Path
from typing import Any

from PyQt6.QtCore import QSettings


class Config:
    """Typed wrapper around :class:`QSettings`."""

    _DEFAULTS: dict[str, Any] = {
        "workspace/width_mm": 400.0,
        "workspace/height_mm": 400.0,
        "workspace/show_grid": True,
        "workspace/grid_spacing_mm": 10.0,
        "app/last_directory": str(Path.home()),
        "app/recent_files": [],
        "app/theme": "dark",
        "device/port": "",
        "device/baud_rate": 115200,
        "device/type": "grbl",
    }

    def __init__(self) -> None:
        self._qs = QSettings("SnartLaser", "SnartLaser")

    # ------------------------------------------------------------------
    # Generic helpers
    # ------------------------------------------------------------------

    def get(self, key: str, default: Any = None) -> Any:
        """Return the stored value for *key*, or *default* if not set."""
        return self._qs.value(key, self._DEFAULTS.get(key, default))

    def set(self, key: str, value: Any) -> None:
        """Persist *value* for *key*."""
        self._qs.setValue(key, value)

    # ------------------------------------------------------------------
    # Workspace
    # ------------------------------------------------------------------

    @property
    def workspace_width_mm(self) -> float:
        return float(self.get("workspace/width_mm"))

    @workspace_width_mm.setter
    def workspace_width_mm(self, value: float) -> None:
        self.set("workspace/width_mm", value)

    @property
    def workspace_height_mm(self) -> float:
        return float(self.get("workspace/height_mm"))

    @workspace_height_mm.setter
    def workspace_height_mm(self, value: float) -> None:
        self.set("workspace/height_mm", value)

    @property
    def show_grid(self) -> bool:
        v = self.get("workspace/show_grid")
        return v if isinstance(v, bool) else str(v).lower() == "true"

    @show_grid.setter
    def show_grid(self, value: bool) -> None:
        self.set("workspace/show_grid", value)

    @property
    def grid_spacing_mm(self) -> float:
        return float(self.get("workspace/grid_spacing_mm"))

    @grid_spacing_mm.setter
    def grid_spacing_mm(self, value: float) -> None:
        self.set("workspace/grid_spacing_mm", value)

    # ------------------------------------------------------------------
    # Application
    # ------------------------------------------------------------------

    @property
    def last_directory(self) -> str:
        return str(self.get("app/last_directory"))

    @last_directory.setter
    def last_directory(self, value: str) -> None:
        self.set("app/last_directory", value)

    @property
    def recent_files(self) -> list[str]:
        v = self.get("app/recent_files")
        if isinstance(v, list):
            return v
        return []

    def add_recent_file(self, path: str, max_entries: int = 10) -> None:
        """Prepend *path* to the recent-files list, keeping at most *max_entries*."""
        files = self.recent_files
        if path in files:
            files.remove(path)
        files.insert(0, path)
        self.set("app/recent_files", files[:max_entries])

    @property
    def theme(self) -> str:
        return str(self.get("app/theme"))

    @theme.setter
    def theme(self, value: str) -> None:
        self.set("app/theme", value)

    # ------------------------------------------------------------------
    # Device
    # ------------------------------------------------------------------

    @property
    def device_port(self) -> str:
        return str(self.get("device/port"))

    @device_port.setter
    def device_port(self, value: str) -> None:
        self.set("device/port", value)

    @property
    def device_baud_rate(self) -> int:
        return int(self.get("device/baud_rate"))

    @device_baud_rate.setter
    def device_baud_rate(self, value: int) -> None:
        self.set("device/baud_rate", value)

    @property
    def device_type(self) -> str:
        return str(self.get("device/type"))

    @device_type.setter
    def device_type(self, value: str) -> None:
        self.set("device/type", value)
