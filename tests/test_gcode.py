"""Tests for snartlaser.gcode.generator."""
from __future__ import annotations

import os

import pytest

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")


def _make_rect_path():
    from PyQt6.QtGui import QPainterPath
    from PyQt6.QtCore import QRectF

    p = QPainterPath()
    p.addRect(QRectF(10, 10, 80, 80))
    return p


def test_gcode_line_mode(qapp):
    from snartlaser.core.types import CutSettings, DeviceType, LayerMode
    from snartlaser.gcode.generator import GCodeGenerator

    gen = GCodeGenerator(DeviceType.GRBL_LASER)
    path = _make_rect_path()
    cs = CutSettings(mode=LayerMode.LINE, speed_mm_s=100, power_pct=80, passes=1)
    lines = gen.generate([([path], cs)])

    joined = "\n".join(lines)
    assert "G21" in joined       # metric
    assert "G90" in joined       # absolute
    assert "M3 S" in joined      # laser on
    assert "G1 " in joined       # cut move
    assert "M5" in joined        # laser off
    assert "M2" in joined        # end of program


def test_gcode_fill_mode(qapp):
    from snartlaser.core.types import CutSettings, DeviceType, LayerMode
    from snartlaser.gcode.generator import GCodeGenerator

    gen = GCodeGenerator(DeviceType.GRBL_LASER)
    path = _make_rect_path()
    cs = CutSettings(mode=LayerMode.FILL, speed_mm_s=200, power_pct=60, passes=1)
    lines = gen.generate([([path], cs)])

    joined = "\n".join(lines)
    # Raster fill should have: laser on, multiple horizontal G1 moves, laser off marker
    assert "raster fill" in joined.lower()
    assert "raster end" in joined.lower()
    # Must have actual cut moves for the fill
    assert "G1 " in joined


def test_gcode_multipass(qapp):
    from snartlaser.core.types import CutSettings, DeviceType, LayerMode
    from snartlaser.gcode.generator import GCodeGenerator

    gen = GCodeGenerator(DeviceType.GRBL_LASER)
    path = _make_rect_path()
    cs = CutSettings(mode=LayerMode.LINE, speed_mm_s=50, power_pct=90, passes=3)
    lines = gen.generate([([path], cs)])
    joined = "\n".join(lines)
    assert "Pass 1/3" in joined
    assert "Pass 3/3" in joined


def test_gcode_disabled_layer(qapp):
    from snartlaser.core.types import CutSettings, DeviceType, LayerMode
    from snartlaser.gcode.generator import GCodeGenerator

    gen = GCodeGenerator(DeviceType.GRBL_LASER)
    path = _make_rect_path()
    cs = CutSettings(enabled=False)
    lines = gen.generate([([path], cs)])
    joined = "\n".join(lines)
    # Should not have any cut moves since layer is disabled
    assert "G1 " not in joined


def test_hpgl_vinyl(qapp):
    from snartlaser.core.types import CutSettings, DeviceType, LayerMode
    from snartlaser.gcode.generator import GCodeGenerator

    gen = GCodeGenerator(DeviceType.VINYL_CUTTER)
    path = _make_rect_path()
    cs = CutSettings(mode=LayerMode.LINE, speed_mm_s=50, power_pct=0)
    lines = gen.generate([([path], cs)])
    joined = "\n".join(lines)
    assert "IN" in joined
    assert "SP1" in joined
    assert "PU" in joined or "PD" in joined


def test_coordinate_inversion(qapp):
    """With origin_bottom_left, Y should be inverted."""
    from snartlaser.core.types import CutSettings, DeviceType, LayerMode
    from snartlaser.gcode.generator import GCodeGenerator

    gen_normal = GCodeGenerator(
        DeviceType.GRBL_LASER, workspace_height_mm=100, origin_bottom_left=False
    )
    gen_inverted = GCodeGenerator(
        DeviceType.GRBL_LASER, workspace_height_mm=100, origin_bottom_left=True
    )
    mx_n, my_n = gen_normal._to_machine(50, 10)
    mx_i, my_i = gen_inverted._to_machine(50, 10)
    assert mx_n == mx_i
    assert my_n != my_i
    assert abs(my_n + my_i - 100) < 0.001


def test_generate_string(qapp):
    from snartlaser.core.types import CutSettings, DeviceType
    from snartlaser.gcode.generator import GCodeGenerator

    gen = GCodeGenerator(DeviceType.GRBL_LASER)
    path = _make_rect_path()
    cs = CutSettings()
    result = gen.generate_string([([path], cs)])
    assert isinstance(result, str)
    assert "\n" in result
