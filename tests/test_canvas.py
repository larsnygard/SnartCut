"""Tests for the canvas scene (no display required – uses offscreen platform)."""
from __future__ import annotations

import os

import pytest

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")


def _make_rect_path():
    from PyQt6.QtGui import QPainterPath
    from PyQt6.QtCore import QRectF

    p = QPainterPath()
    p.addRect(QRectF(0, 0, 50, 50))
    return p


def test_scene_add_path(qapp):
    from snartlaser.canvas.scene import DesignScene

    scene = DesignScene(100, 100)
    item = scene.add_path(_make_rect_path(), "#ff0000")
    assert item is not None
    assert item.item_id in [i.item_id for i in scene.all_design_items()]


def test_scene_remove_item(qapp):
    from snartlaser.canvas.scene import DesignScene

    scene = DesignScene(100, 100)
    item = scene.add_path(_make_rect_path())
    iid = item.item_id
    scene.remove_item(iid)
    assert scene.item_by_id(iid) is None


def test_scene_add_multiple_paths(qapp):
    from PyQt6.QtGui import QPainterPath
    from PyQt6.QtCore import QRectF
    from snartlaser.canvas.scene import DesignScene

    scene = DesignScene(200, 200)
    paths = [(QPainterPath(), "#ff0000"), (QPainterPath(), "#00ff00")]
    for p, c in paths:
        p.addRect(QRectF(0, 0, 10, 10))
    items = scene.add_paths(paths)
    assert len(items) == 2
    assert len(scene.all_design_items()) == 2


def test_scene_serialisation(qapp):
    from snartlaser.canvas.scene import DesignScene

    scene = DesignScene(150, 150)
    scene.add_path(_make_rect_path(), "#ff0000")
    scene.add_path(_make_rect_path(), "#0000ff")

    d = scene.to_dict()
    assert d["workspace_width_mm"] == 150
    assert len(d["items"]) == 2

    scene2 = DesignScene(100, 100)
    scene2.load_dict(d)
    assert scene2.workspace_width_mm == 150
    assert len(scene2.all_design_items()) == 2


def test_scene_select_all(qapp):
    from snartlaser.canvas.scene import DesignScene

    scene = DesignScene(100, 100)
    scene.add_path(_make_rect_path())
    scene.add_path(_make_rect_path())
    scene.select_all()
    assert len(scene.selected_design_items()) == 2


def test_design_item_color(qapp):
    from snartlaser.canvas.items import DesignItem

    item = DesignItem(_make_rect_path(), "#ff0000")
    assert item.layer_color == "#ff0000"
    item.set_color("#00ff00")
    assert item.layer_color == "#00ff00"


def test_design_item_serialisation(qapp):
    from snartlaser.canvas.items import DesignItem

    item = DesignItem(_make_rect_path(), "#abcdef")
    d = item.to_dict()
    assert d["color"] == "#abcdef"
    assert d["item_id"] == item.item_id

    item2 = DesignItem.from_dict(d)
    assert item2.item_id == item.item_id
    assert item2.layer_color == "#abcdef"


def test_tool_switch(qapp):
    from snartlaser.canvas.scene import DesignScene
    from snartlaser.core.types import ToolType

    scene = DesignScene(100, 100)
    scene.set_tool(ToolType.RECTANGLE)
    assert scene.active_tool.tool_type == ToolType.RECTANGLE

    scene.set_tool(ToolType.SELECT)
    assert scene.active_tool.tool_type == ToolType.SELECT
