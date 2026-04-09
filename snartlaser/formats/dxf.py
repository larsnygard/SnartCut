"""DXF import and export helpers using the `ezdxf <https://ezdxf.readthedocs.io>`_ library.

Import
------
:func:`load` reads a DXF file (R12 through R2018) and converts all supported
entities to :class:`~PyQt6.QtGui.QPainterPath` objects in millimetres.

Export
------
:func:`save` writes a list of painter-paths back to a DXF R2010 file.

Supported entity types during import:

* ``LINE``
* ``LWPOLYLINE`` / ``POLYLINE``
* ``CIRCLE``
* ``ARC``
* ``ELLIPSE``
* ``SPLINE``
"""
from __future__ import annotations

import math
from pathlib import Path
from typing import List, Tuple

import ezdxf
from ezdxf.document import Drawing
from ezdxf.layouts import BaseLayout
from PyQt6.QtGui import QPainterPath
from PyQt6.QtCore import QRectF


def _arc_to_path(
    cx: float, cy: float, r: float, start_deg: float, end_deg: float
) -> QPainterPath:
    """Approximate a DXF ARC as a :class:`QPainterPath`."""
    path = QPainterPath()
    # Qt's arcTo uses span angles and measures from 3 o'clock counter-clockwise,
    # while DXF arcs also go counter-clockwise from 3 o'clock.
    rect = QRectF(cx - r, cy - r, 2 * r, 2 * r)
    span = end_deg - start_deg
    if span <= 0:
        span += 360.0
    path.arcMoveTo(rect, start_deg)
    path.arcTo(rect, start_deg, span)
    return path


def _spline_to_path(entity) -> QPainterPath:
    """Convert a SPLINE entity to a QPainterPath via point approximation."""
    path = QPainterPath()
    try:
        pts = list(entity.flattening(0.01))  # tolerance in drawing units
    except Exception:
        pts = []
    if not pts:
        return path
    path.moveTo(pts[0][0], pts[0][1])
    for p in pts[1:]:
        path.lineTo(p[0], p[1])
    return path


def _entity_to_path(entity) -> QPainterPath | None:
    """Convert a single DXF entity to a :class:`QPainterPath`."""
    dxftype = entity.dxftype()
    path = QPainterPath()

    if dxftype == "LINE":
        s = entity.dxf.start
        e = entity.dxf.end
        path.moveTo(s.x, s.y)
        path.lineTo(e.x, e.y)

    elif dxftype == "CIRCLE":
        c = entity.dxf.center
        r = entity.dxf.radius
        path.addEllipse(QRectF(c.x - r, c.y - r, 2 * r, 2 * r))

    elif dxftype == "ARC":
        c = entity.dxf.center
        r = entity.dxf.radius
        start = entity.dxf.start_angle
        end = entity.dxf.end_angle
        return _arc_to_path(c.x, c.y, r, start, end)

    elif dxftype == "ELLIPSE":
        c = entity.dxf.center
        major = entity.dxf.major_axis
        ratio = entity.dxf.ratio
        a = math.hypot(major.x, major.y)
        b = a * ratio
        angle = math.degrees(math.atan2(major.y, major.x))
        path.addEllipse(QRectF(c.x - a, c.y - b, 2 * a, 2 * b))
        # TODO: apply rotation *angle* around center

    elif dxftype in ("LWPOLYLINE", "POLYLINE"):
        try:
            pts = list(entity.vertices_in_wcs()) if dxftype == "POLYLINE" else list(entity.get_points())
        except Exception:
            try:
                pts = list(entity.vertices())
            except Exception:
                pts = []
        if pts:
            first = pts[0]
            path.moveTo(float(first[0]), float(first[1]))
            for pt in pts[1:]:
                path.lineTo(float(pt[0]), float(pt[1]))
            try:
                if entity.closed or entity.dxf.flags & 1:
                    path.closeSubpath()
            except Exception:
                pass

    elif dxftype == "SPLINE":
        return _spline_to_path(entity)

    else:
        return None

    return path if not path.isEmpty() else None


def _color_for_layer(layer_name: str, doc: Drawing) -> str:
    """Look up the ACI colour for *layer_name* and approximate it as hex."""
    try:
        layer = doc.layers.get(layer_name)
        color_index = abs(layer.dxf.color)
    except Exception:
        color_index = 7  # white

    # Rough ACI index → RGB mapping for common indices
    ACI = {
        1: "#ff0000", 2: "#ffff00", 3: "#00ff00",
        4: "#00ffff", 5: "#0000ff", 6: "#ff00ff",
        7: "#ffffff", 8: "#808080", 9: "#c0c0c0",
    }
    return ACI.get(color_index, "#000000")


def load(path: str | Path) -> List[Tuple[QPainterPath, str]]:
    """Parse a DXF file and return a list of ``(path_mm, color_hex)`` tuples.

    All coordinates in the returned paths use the DXF document's native units
    (typically millimetres for metric drawings).

    Args:
        path: Filesystem path to the ``.dxf`` file.

    Returns:
        List of ``(QPainterPath, color_hex)`` tuples.
    """
    doc = ezdxf.readfile(str(path))
    msp = doc.modelspace()
    results: List[Tuple[QPainterPath, str]] = []

    for entity in msp:
        p = _entity_to_path(entity)
        if p:
            try:
                layer_name = entity.dxf.layer
                color = _color_for_layer(layer_name, doc)
            except Exception:
                color = "#000000"
            results.append((p, color))

    return results


def save(
    paths: List[Tuple[QPainterPath, str]],
    output_path: str | Path,
) -> None:
    """Write *paths* to a DXF R2010 file at *output_path*.

    Args:
        paths:       List of ``(QPainterPath, stroke_color)`` tuples.
        output_path: Destination file path.
    """
    doc = ezdxf.new("R2010", setup=True)
    msp = doc.modelspace()

    for qpath, color in paths:
        # Convert QPainterPath segments to DXF LINE entities
        prev = None
        start = None
        for i in range(qpath.elementCount()):
            elem = qpath.elementAt(i)
            from PyQt6.QtGui import QPainterPath as _QP
            etype = elem.type
            pt = (elem.x, elem.y)

            if etype == _QP.ElementType.MoveToElement:
                start = pt
                prev = pt
            elif etype == _QP.ElementType.LineToElement and prev is not None:
                msp.add_line(prev, pt)
                prev = pt
            elif etype == _QP.ElementType.CurveToElement:
                # Approximate cubic bezier with lines (simplified)
                prev = pt  # just move forward
            elif etype == _QP.ElementType.CurveToDataElement:
                if prev:
                    msp.add_line(prev, pt)
                    prev = pt

    doc.saveas(str(output_path))
