"""SVG import and export helpers.

Import
------
:func:`load` reads an SVG file and returns a list of
:class:`~PyQt6.QtGui.QPainterPath` objects ready to be placed on the canvas.

Export
------
:func:`save` serialises the given painter-paths back to an SVG file.

All coordinates are in *millimetres* (converted from the SVG user-unit system
using the document's ``viewBox`` and ``width``/``height`` attributes).
"""
from __future__ import annotations

import re
from pathlib import Path
from typing import List, Tuple
from xml.etree import ElementTree as ET

from PyQt6.QtGui import QPainterPath, QTransform
from PyQt6.QtCore import QRectF

# SVG namespace
_NS = "http://www.w3.org/2000/svg"
_SVG_NS = {"svg": _NS}

# Points per inch (SVG spec: 1 in = 96 px = 25.4 mm)
_PX_PER_MM = 96.0 / 25.4


def _unit_to_mm(value: str, viewport_mm: float = 100.0) -> float:
    """Convert an SVG length string to millimetres."""
    value = value.strip()
    if value.endswith("mm"):
        return float(value[:-2])
    if value.endswith("cm"):
        return float(value[:-2]) * 10.0
    if value.endswith("in"):
        return float(value[:-2]) * 25.4
    if value.endswith("pt"):
        return float(value[:-2]) * 25.4 / 72.0
    if value.endswith("pc"):
        return float(value[:-2]) * 25.4 / 6.0
    if value.endswith("%"):
        return float(value[:-1]) / 100.0 * viewport_mm
    # px or unitless
    num_str = re.sub(r"[^0-9.\-]", "", value)
    try:
        return float(num_str) / _PX_PER_MM
    except ValueError:
        return 0.0


def _parse_transform(transform: str) -> QTransform:
    """Parse an SVG ``transform`` attribute into a :class:`QTransform`."""
    t = QTransform()
    if not transform:
        return t
    for m in re.finditer(
        r"(\w+)\(([^)]+)\)", transform
    ):
        func = m.group(1)
        args = [float(a) for a in re.split(r"[,\s]+", m.group(2).strip()) if a]
        if func == "translate":
            dx = args[0] if args else 0.0
            dy = args[1] if len(args) > 1 else 0.0
            t = t * QTransform.fromTranslate(dx, dy)
        elif func == "scale":
            sx = args[0] if args else 1.0
            sy = args[1] if len(args) > 1 else sx
            t = t * QTransform.fromScale(sx, sy)
        elif func == "rotate":
            angle = args[0] if args else 0.0
            cx = args[1] if len(args) > 1 else 0.0
            cy = args[2] if len(args) > 2 else 0.0
            t = t * (
                QTransform.fromTranslate(cx, cy)
                * QTransform().rotate(angle)
                * QTransform.fromTranslate(-cx, -cy)
            )
        elif func == "matrix":
            if len(args) >= 6:
                t = t * QTransform(args[0], args[1], args[2], args[3], args[4], args[5])
    return t


def _path_from_d(d: str) -> QPainterPath:
    """Convert an SVG path ``d`` attribute string to a :class:`QPainterPath`."""
    path = QPainterPath()
    # Tokenise
    tokens = re.findall(r"[MmZzLlHhVvCcSsQqTtAa]|[-+]?[0-9]*\.?[0-9]+(?:[eE][-+]?[0-9]+)?", d)
    idx = 0
    cmd = ""
    cx, cy = 0.0, 0.0  # current position
    last_cp: tuple[float, float] | None = None  # last control point for S/T

    def next_val() -> float:
        nonlocal idx
        v = float(tokens[idx])
        idx += 1
        return v

    while idx < len(tokens):
        token = tokens[idx]
        if token.isalpha() or token == "-":
            if token.isalpha():
                cmd = token
                idx += 1
        try:
            if cmd in ("M", "m"):
                x, y = next_val(), next_val()
                if cmd == "m":
                    x += cx
                    y += cy
                path.moveTo(x, y)
                cx, cy = x, y
                cmd = "L" if cmd == "M" else "l"
            elif cmd in ("L", "l"):
                x, y = next_val(), next_val()
                if cmd == "l":
                    x += cx
                    y += cy
                path.lineTo(x, y)
                cx, cy = x, y
            elif cmd in ("H", "h"):
                x = next_val()
                if cmd == "h":
                    x += cx
                path.lineTo(x, cy)
                cx = x
            elif cmd in ("V", "v"):
                y = next_val()
                if cmd == "v":
                    y += cy
                path.lineTo(cx, y)
                cy = y
            elif cmd in ("Z", "z"):
                path.closeSubpath()
            elif cmd in ("C", "c"):
                x1, y1 = next_val(), next_val()
                x2, y2 = next_val(), next_val()
                x, y = next_val(), next_val()
                if cmd == "c":
                    x1 += cx
                    y1 += cy
                    x2 += cx
                    y2 += cy
                    x += cx
                    y += cy
                last_cp = (x2, y2)
                path.cubicTo(x1, y1, x2, y2, x, y)
                cx, cy = x, y
            elif cmd in ("S", "s"):
                x2, y2 = next_val(), next_val()
                x, y = next_val(), next_val()
                if cmd == "s":
                    x2 += cx
                    y2 += cy
                    x += cx
                    y += cy
                if last_cp:
                    x1 = 2 * cx - last_cp[0]
                    y1 = 2 * cy - last_cp[1]
                else:
                    x1, y1 = cx, cy
                last_cp = (x2, y2)
                path.cubicTo(x1, y1, x2, y2, x, y)
                cx, cy = x, y
            elif cmd in ("Q", "q"):
                x1, y1 = next_val(), next_val()
                x, y = next_val(), next_val()
                if cmd == "q":
                    x1 += cx
                    y1 += cy
                    x += cx
                    y += cy
                last_cp = (x1, y1)
                path.quadTo(x1, y1, x, y)
                cx, cy = x, y
            elif cmd in ("T", "t"):
                x, y = next_val(), next_val()
                if cmd == "t":
                    x += cx
                    y += cy
                if last_cp:
                    x1 = 2 * cx - last_cp[0]
                    y1 = 2 * cy - last_cp[1]
                else:
                    x1, y1 = cx, cy
                last_cp = (x1, y1)
                path.quadTo(x1, y1, x, y)
                cx, cy = x, y
            else:
                idx += 1  # skip unknown
        except (IndexError, ValueError):
            break
    return path


def _rect_to_path(elem: ET.Element, scale: float) -> QPainterPath:
    x = float(elem.get("x", 0)) * scale
    y = float(elem.get("y", 0)) * scale
    w = float(elem.get("width", 0)) * scale
    h = float(elem.get("height", 0)) * scale
    rx = float(elem.get("rx", elem.get("ry", 0))) * scale
    path = QPainterPath()
    if rx:
        path.addRoundedRect(QRectF(x, y, w, h), rx, rx)
    else:
        path.addRect(QRectF(x, y, w, h))
    return path


def _circle_to_path(elem: ET.Element, scale: float) -> QPainterPath:
    cx = float(elem.get("cx", 0)) * scale
    cy = float(elem.get("cy", 0)) * scale
    r = float(elem.get("r", 0)) * scale
    path = QPainterPath()
    path.addEllipse(QRectF(cx - r, cy - r, 2 * r, 2 * r))
    return path


def _ellipse_to_path(elem: ET.Element, scale: float) -> QPainterPath:
    cx = float(elem.get("cx", 0)) * scale
    cy = float(elem.get("cy", 0)) * scale
    rx = float(elem.get("rx", 0)) * scale
    ry = float(elem.get("ry", 0)) * scale
    path = QPainterPath()
    path.addEllipse(QRectF(cx - rx, cy - ry, 2 * rx, 2 * ry))
    return path


def _line_to_path(elem: ET.Element, scale: float) -> QPainterPath:
    x1 = float(elem.get("x1", 0)) * scale
    y1 = float(elem.get("y1", 0)) * scale
    x2 = float(elem.get("x2", 0)) * scale
    y2 = float(elem.get("y2", 0)) * scale
    path = QPainterPath()
    path.moveTo(x1, y1)
    path.lineTo(x2, y2)
    return path


def _polyline_to_path(elem: ET.Element, scale: float, close: bool = False) -> QPainterPath:
    pts_str = elem.get("points", "")
    pts = [float(v) for v in re.split(r"[,\s]+", pts_str.strip()) if v]
    path = QPainterPath()
    if len(pts) >= 2:
        path.moveTo(pts[0] * scale, pts[1] * scale)
        for i in range(2, len(pts) - 1, 2):
            path.lineTo(pts[i] * scale, pts[i + 1] * scale)
        if close:
            path.closeSubpath()
    return path


def _elem_to_paths(
    elem: ET.Element,
    scale: float,
    parent_transform: QTransform | None = None,
) -> List[Tuple[QPainterPath, str]]:
    """Recursively convert SVG elements to (QPainterPath, stroke_color) tuples."""
    results: List[Tuple[QPainterPath, str]] = []
    tag = elem.tag.replace(f"{{{_NS}}}", "")
    transform_str = elem.get("transform", "")
    local_t = _parse_transform(transform_str)
    t = (parent_transform * local_t) if parent_transform else local_t

    stroke = elem.get("stroke", elem.get("style", ""))
    # extract stroke from inline style
    m = re.search(r"stroke\s*:\s*(#[0-9a-fA-F]+|[a-z]+)", stroke)
    color = m.group(1) if m else "#000000"
    fill = elem.get("fill", "")
    if not color or color == "none":
        fm = re.search(r"fill\s*:\s*(#[0-9a-fA-F]+|[a-z]+)", fill)
        color = fm.group(1) if fm else "#000000"

    path: QPainterPath | None = None

    if tag == "path":
        d = elem.get("d", "")
        if d:
            path = _path_from_d(d)
    elif tag == "rect":
        path = _rect_to_path(elem, scale)
    elif tag == "circle":
        path = _circle_to_path(elem, scale)
    elif tag == "ellipse":
        path = _ellipse_to_path(elem, scale)
    elif tag == "line":
        path = _line_to_path(elem, scale)
    elif tag == "polyline":
        path = _polyline_to_path(elem, scale, close=False)
    elif tag == "polygon":
        path = _polyline_to_path(elem, scale, close=True)
    elif tag in ("g", "svg"):
        for child in elem:
            results.extend(_elem_to_paths(child, scale, t))
        return results

    if path and not path.isEmpty():
        # Apply coordinate transform (SVG uses px; we've already scaled to mm)
        if t and not t.isIdentity():
            path = t.map(path)
        results.append((path, color))

    return results


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def load(path: str | Path) -> List[Tuple[QPainterPath, str]]:
    """Parse an SVG file and return a list of ``(path_mm, stroke_color)`` tuples.

    All coordinates in the returned paths are in millimetres.

    Args:
        path: Filesystem path to the ``.svg`` file.

    Returns:
        List of ``(QPainterPath, color_hex)`` tuples.
    """
    tree = ET.parse(str(path))
    root = tree.getroot()

    # Determine scale: px → mm
    width_str = root.get("width", "100mm")
    height_str = root.get("height", "100mm")
    width_mm = _unit_to_mm(width_str)
    height_mm = _unit_to_mm(height_str)

    viewbox = root.get("viewBox")
    if viewbox:
        parts = [float(v) for v in re.split(r"[,\s]+", viewbox.strip()) if v]
        if len(parts) >= 4:
            vb_w = parts[2]
            vb_h = parts[3]
            scale_x = width_mm / vb_w if vb_w else 1.0
            scale_y = height_mm / vb_h if vb_h else 1.0
            scale = (scale_x + scale_y) / 2.0
        else:
            scale = 1.0 / _PX_PER_MM
    else:
        scale = 1.0 / _PX_PER_MM

    return _elem_to_paths(root, scale)


def save(
    paths: List[Tuple[QPainterPath, str]],
    width_mm: float,
    height_mm: float,
    output_path: str | Path,
) -> None:
    """Write *paths* to an SVG file at *output_path*.

    Args:
        paths:       List of ``(QPainterPath, stroke_color)`` tuples.
        width_mm:    Canvas width in millimetres.
        height_mm:   Canvas height in millimetres.
        output_path: Destination file path.
    """
    ET.register_namespace("", _NS)
    root = ET.Element(f"{{{_NS}}}svg")
    root.set("xmlns", _NS)
    root.set("width", f"{width_mm}mm")
    root.set("height", f"{height_mm}mm")
    root.set("viewBox", f"0 0 {width_mm} {height_mm}")

    for qpath, color in paths:
        # Convert QPainterPath to SVG path d attribute
        d_parts = []
        for i in range(qpath.elementCount()):
            elem = qpath.elementAt(i)
            etype = elem.type
            from PyQt6.QtGui import QPainterPath as _QP
            if etype == _QP.ElementType.MoveToElement:
                d_parts.append(f"M {elem.x:.4f} {elem.y:.4f}")
            elif etype == _QP.ElementType.LineToElement:
                d_parts.append(f"L {elem.x:.4f} {elem.y:.4f}")
            elif etype == _QP.ElementType.CurveToElement:
                d_parts.append(f"C {elem.x:.4f} {elem.y:.4f}")
            elif etype == _QP.ElementType.CurveToDataElement:
                d_parts.append(f"{elem.x:.4f} {elem.y:.4f}")
        if d_parts:
            path_elem = ET.SubElement(root, f"{{{_NS}}}path")
            path_elem.set("d", " ".join(d_parts))
            path_elem.set("stroke", color)
            path_elem.set("fill", "none")
            path_elem.set("stroke-width", "0.2")

    tree = ET.ElementTree(root)
    ET.indent(tree, space="  ")
    tree.write(str(output_path), encoding="utf-8", xml_declaration=True)
