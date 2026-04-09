"""G-code and HPGL generator.

:class:`GCodeGenerator` converts a list of
:class:`~PyQt6.QtGui.QPainterPath` objects and their associated
:class:`~snartlaser.core.types.CutSettings` into a list of command strings
that can be sent to a machine.

GRBL laser mode (``$32=1``) is assumed by default.  The generator emits:

* ``M3 S<power>`` / ``M5`` for laser on/off
* ``G0`` for rapid moves (laser off)
* ``G1 F<feed> ...`` for cut moves (laser on)
* ``M8`` / ``M9`` for air-assist (if supported)

For vinyl cutters (HPGL) the generator emits ``PU`` / ``PD`` pen-up/down
commands instead.
"""
from __future__ import annotations

import math
from typing import List, Tuple

from PyQt6.QtGui import QPainterPath

from snartlaser.core.types import AirAssist, CutSettings, DeviceType, LayerMode

# Resolution: how many line segments to use when flattening curves (mm)
_FLATTEN_TOLERANCE_MM = 0.05

# HPGL plotter units = 1/40 mm
_HPGL_UNITS_PER_MM = 40.0


class GCodeGenerator:
    """Generate machine code from canvas paths and cut settings.

    Args:
        device_type:  Target machine type.  Defaults to
            :attr:`~snartlaser.core.types.DeviceType.GRBL_LASER`.
        workspace_width_mm:  Work area width (used for coordinate inversion).
        workspace_height_mm: Work area height.
        origin_bottom_left:  If ``True`` the Y axis is inverted so that the
            machine origin sits at the bottom-left corner (LightBurn style).
    """

    def __init__(
        self,
        device_type: DeviceType = DeviceType.GRBL_LASER,
        workspace_width_mm: float = 400.0,
        workspace_height_mm: float = 400.0,
        origin_bottom_left: bool = True,
    ) -> None:
        self.device_type = device_type
        self.workspace_width_mm = workspace_width_mm
        self.workspace_height_mm = workspace_height_mm
        self.origin_bottom_left = origin_bottom_left

    # ------------------------------------------------------------------
    # Public interface
    # ------------------------------------------------------------------

    def generate(
        self,
        layers: List[Tuple[List[QPainterPath], CutSettings]],
    ) -> List[str]:
        """Generate machine code lines for *layers*.

        Args:
            layers: List of ``(paths, cut_settings)`` pairs ordered from
                    bottom to top.

        Returns:
            A list of command strings (without trailing newlines).
        """
        if self.device_type == DeviceType.VINYL_CUTTER:
            return self._generate_hpgl(layers)
        return self._generate_gcode(layers)

    def generate_string(
        self,
        layers: List[Tuple[List[QPainterPath], CutSettings]],
    ) -> str:
        """Like :meth:`generate` but returns a single newline-joined string."""
        return "\n".join(self.generate(layers))

    # ------------------------------------------------------------------
    # G-code (GRBL laser / Marlin)
    # ------------------------------------------------------------------

    def _generate_gcode(
        self, layers: List[Tuple[List[QPainterPath], CutSettings]]
    ) -> List[str]:
        lines: List[str] = []

        # Preamble
        lines += [
            "; SnartLaser generated G-code",
            "G21       ; units mm",
            "G90       ; absolute positioning",
            "G0 X0 Y0  ; home",
            "M5        ; laser off",
        ]

        for paths, settings in layers:
            if not settings.enabled:
                continue

            feed = settings.speed_mm_s * 60  # mm/s → mm/min
            power = int(settings.power_pct / 100.0 * 1000)  # 0–1000 S range (GRBL)

            lines.append(f"; Layer: {settings.name}")

            if settings.air_assist == AirAssist.ON:
                lines.append("M8  ; air assist on")

            for _pass in range(settings.passes):
                if settings.passes > 1:
                    lines.append(f"; Pass {_pass + 1}/{settings.passes}")

                for path in paths:
                    if settings.mode == LayerMode.FILL:
                        lines.extend(
                            self._raster_fill_gcode(path, settings, feed, power)
                        )
                    else:
                        lines.extend(
                            self._vector_cut_gcode(path, settings, feed, power)
                        )

            if settings.air_assist == AirAssist.ON:
                lines.append("M9  ; air assist off")

        # Postamble
        lines += [
            "M5        ; laser off",
            "G0 X0 Y0  ; return home",
            "M2        ; end of program",
        ]
        return lines

    def _to_machine(self, x: float, y: float) -> Tuple[float, float]:
        """Convert canvas coordinates (mm, Y-down) to machine coordinates."""
        if self.origin_bottom_left:
            y = self.workspace_height_mm - y
        return round(x, 3), round(y, 3)

    def _vector_cut_gcode(
        self,
        path: QPainterPath,
        settings: CutSettings,
        feed_mm_min: float,
        power: int,
    ) -> List[str]:
        lines: List[str] = []
        pts = self._flatten_path(path)
        if not pts:
            return lines

        mx, my = self._to_machine(*pts[0])
        lines.append(f"G0 X{mx} Y{my}  ; rapid to start")
        lines.append(f"M3 S{power}  ; laser on")

        for px, py in pts[1:]:
            mx, my = self._to_machine(px, py)
            lines.append(f"G1 X{mx} Y{my} F{feed_mm_min:.0f}")

        lines.append("M5  ; laser off")
        return lines

    def _raster_fill_gcode(
        self,
        path: QPainterPath,
        settings: CutSettings,
        feed_mm_min: float,
        power: int,
    ) -> List[str]:
        """Simple horizontal raster fill using bounding box."""
        lines: List[str] = []
        bb = path.boundingRect()
        if bb.isEmpty():
            return lines

        line_spacing = 0.1  # mm between raster lines
        y = bb.top()

        direction = 1
        lines.append(f"M3 S{power}  ; laser on (raster fill)")

        while y <= bb.bottom():
            mx_start, my = self._to_machine(bb.left() if direction == 1 else bb.right(), y)
            mx_end, _ = self._to_machine(bb.right() if direction == 1 else bb.left(), y)
            lines.append(f"G0 X{mx_start} Y{my}")
            lines.append(f"G1 X{mx_end} Y{my} F{feed_mm_min:.0f}")
            y += line_spacing
            direction *= -1

        lines.append("M5  ; laser off (raster end)")
        return lines

    def _flatten_path(self, path: QPainterPath) -> List[Tuple[float, float]]:
        """Flatten a QPainterPath to a list of (x, y) tuples (mm)."""
        from PyQt6.QtGui import QPainterPath as _QP
        points: List[Tuple[float, float]] = []

        for i in range(path.elementCount()):
            elem = path.elementAt(i)
            etype = elem.type
            pt = (elem.x, elem.y)

            if etype == _QP.ElementType.MoveToElement:
                points.append(pt)
            elif etype == _QP.ElementType.LineToElement:
                points.append(pt)
            elif etype in (
                _QP.ElementType.CurveToElement,
                _QP.ElementType.CurveToDataElement,
            ):
                points.append(pt)

        return points

    # ------------------------------------------------------------------
    # HPGL (vinyl cutters)
    # ------------------------------------------------------------------

    def _generate_hpgl(
        self, layers: List[Tuple[List[QPainterPath], CutSettings]]
    ) -> List[str]:
        lines: List[str] = [
            "IN;  Initialize",
            "SP1; Select pen 1",
        ]

        for paths, settings in layers:
            if not settings.enabled:
                continue
            lines.append(f"; Layer: {settings.name}")
            for path in paths:
                lines.extend(self._path_to_hpgl(path))

        lines.append("PU0,0;  Pen up, move home")
        lines.append("SP0;  Deselect pen")
        lines.append("IN;  End")
        return lines

    def _path_to_hpgl(self, path: QPainterPath) -> List[str]:
        from PyQt6.QtGui import QPainterPath as _QP
        lines: List[str] = []

        pen_down = False
        for i in range(path.elementCount()):
            elem = path.elementAt(i)
            etype = elem.type
            hx = int(elem.x * _HPGL_UNITS_PER_MM)
            hy = int(elem.y * _HPGL_UNITS_PER_MM)

            if etype == _QP.ElementType.MoveToElement:
                if pen_down:
                    lines.append(f"PU{hx},{hy};")
                    pen_down = False
                else:
                    lines.append(f"PU{hx},{hy};")
            elif etype in (
                _QP.ElementType.LineToElement,
                _QP.ElementType.CurveToElement,
                _QP.ElementType.CurveToDataElement,
            ):
                if not pen_down:
                    lines.append(f"PD{hx},{hy};")
                    pen_down = True
                else:
                    lines.append(f"PA{hx},{hy};")

        if pen_down:
            lines.append("PU;")
        return lines
