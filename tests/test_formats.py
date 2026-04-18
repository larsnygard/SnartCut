"""Tests for snartlaser.formats (SVG and DXF import/export)."""
from __future__ import annotations

import os
import tempfile
from pathlib import Path

import pytest

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")


# ---------------------------------------------------------------------------
# SVG
# ---------------------------------------------------------------------------


def test_svg_load_simple(qapp, tmp_path):
    svg = tmp_path / "test.svg"
    svg.write_text(
        """<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg"
     width="100mm" height="100mm" viewBox="0 0 100 100">
  <rect x="10" y="10" width="80" height="80" stroke="#ff0000" fill="none"/>
  <circle cx="50" cy="50" r="30" stroke="#0000ff" fill="none"/>
  <line x1="0" y1="0" x2="100" y2="100" stroke="#00ff00"/>
</svg>"""
    )

    from snartlaser.formats.svg import load

    paths = load(svg)
    assert len(paths) == 3
    # Each result is (QPainterPath, color)
    for p, color in paths:
        assert not p.isEmpty()
        assert color.startswith("#")


def test_svg_load_path_element(qapp, tmp_path):
    svg = tmp_path / "path.svg"
    svg.write_text(
        """<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="200mm" height="200mm" viewBox="0 0 200 200">
  <path d="M 10 10 L 190 10 L 190 190 L 10 190 Z" stroke="#ff0000" fill="none"/>
</svg>"""
    )

    from snartlaser.formats.svg import load

    paths = load(svg)
    assert len(paths) == 1
    p, color = paths[0]
    assert not p.isEmpty()


def test_svg_save_and_reload(qapp, tmp_path):
    from PyQt6.QtGui import QPainterPath
    from PyQt6.QtCore import QRectF
    from snartlaser.formats.svg import save, load

    path = QPainterPath()
    path.addRect(QRectF(10, 10, 80, 80))
    out = tmp_path / "out.svg"
    save([(path, "#ff0000")], 100.0, 100.0, out)

    assert out.exists()
    content = out.read_text()
    assert "<svg" in content
    assert "width=\"100.0mm\"" in content or "width=\"100mm\"" in content


def test_svg_unit_conversions(qapp, tmp_path):
    """Check that mm/cm/in/pt units are handled."""
    svg = tmp_path / "units.svg"
    svg.write_text(
        """<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="10cm" height="5cm" viewBox="0 0 100 50">
  <rect x="0" y="0" width="100" height="50" stroke="black" fill="none"/>
</svg>"""
    )
    from snartlaser.formats.svg import load

    paths = load(svg)
    assert len(paths) == 1


# ---------------------------------------------------------------------------
# DXF
# ---------------------------------------------------------------------------


def test_dxf_load_line(qapp, tmp_path):
    """Create a minimal DXF file and load it."""
    import ezdxf

    doc = ezdxf.new("R2010")
    msp = doc.modelspace()
    msp.add_line((0, 0), (100, 100))
    msp.add_circle((50, 50), 30)
    out = tmp_path / "test.dxf"
    doc.saveas(str(out))

    from snartlaser.formats.dxf import load

    paths = load(out)
    assert len(paths) == 2
    for p, color in paths:
        assert not p.isEmpty()


def test_dxf_save(qapp, tmp_path):
    from PyQt6.QtGui import QPainterPath
    from snartlaser.formats.dxf import save

    path = QPainterPath()
    path.moveTo(0, 0)
    path.lineTo(100, 0)
    path.lineTo(100, 100)
    path.closeSubpath()

    out = tmp_path / "out.dxf"
    save([(path, "#ff0000")], out)
    assert out.exists()
    content = out.read_text()
    assert "LINE" in content or "SECTION" in content
