"""Tests for snartlaser.job (layer, settings, material library)."""
from __future__ import annotations

import json
import os
import tempfile

import pytest

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")


def test_layer_list_add_remove():
    from snartlaser.job.layer import LayerList

    ll = LayerList()
    assert len(ll) == 0

    l1 = ll.add("Cut")
    l2 = ll.add("Engrave")
    assert len(ll) == 2
    assert ll[0].name == "Cut"
    assert ll[1].name == "Engrave"

    ll.remove(0)
    assert len(ll) == 1
    assert ll[0].name == "Engrave"


def test_layer_list_move():
    from snartlaser.job.layer import LayerList

    ll = LayerList()
    ll.add("A")
    ll.add("B")
    ll.add("C")
    ll.move(0, 2)
    assert ll[0].name == "B"
    assert ll[1].name == "C"
    assert ll[2].name == "A"


def test_layer_item_assignment():
    from snartlaser.job.layer import LayerList

    ll = LayerList()
    ll.add("Layer 1")
    ll.add("Layer 2")

    ll.assign_item("item-abc", 0)
    assert "item-abc" in ll[0].item_ids

    ll.assign_item("item-abc", 1)
    assert "item-abc" not in ll[0].item_ids
    assert "item-abc" in ll[1].item_ids


def test_layer_find_by_item():
    from snartlaser.job.layer import LayerList

    ll = LayerList()
    ll.add("Layer 1")
    ll.add("Layer 2")
    ll.assign_item("x-123", 1)

    found = ll.find_by_item("x-123")
    assert found is not None
    assert found.name == "Layer 2"

    assert ll.find_by_item("nonexistent") is None


def test_layer_serialisation():
    from snartlaser.job.layer import LayerList
    from snartlaser.core.types import CutSettings, LayerMode

    ll = LayerList()
    l = ll.add("Plywood Cut")
    l.settings.mode = LayerMode.LINE
    l.settings.speed_mm_s = 30.0
    l.settings.power_pct = 90.0
    ll.assign_item("item-1", 0)

    d = ll.to_dict()
    ll2 = LayerList.from_dict(d)
    assert len(ll2) == 1
    assert ll2[0].name == "Plywood Cut"
    assert ll2[0].settings.speed_mm_s == 30.0
    assert "item-1" in ll2[0].item_ids


def test_job_settings_save_load(tmp_path):
    from snartlaser.job.settings import JobSettings

    js = JobSettings()
    js.workspace.width_mm = 300.0
    js.workspace.height_mm = 200.0
    js.material = "Plywood"
    js.notes = "Test notes"
    js.layers.add("Cut")

    out = tmp_path / "test.json"
    js.save(out)
    assert out.exists()

    js2 = JobSettings.load(out)
    assert js2.workspace.width_mm == 300.0
    assert js2.workspace.height_mm == 200.0
    assert js2.material == "Plywood"
    assert js2.notes == "Test notes"
    assert len(js2.layers) == 1


def test_material_library_keys():
    from snartlaser.job.settings import MATERIAL_LIBRARY

    assert len(MATERIAL_LIBRARY) > 0
    # Check that vinyl is present
    vinyl_keys = [k for k in MATERIAL_LIBRARY if "vinyl" in k.lower()]
    assert len(vinyl_keys) >= 1


def test_apply_preset():
    from snartlaser.job.settings import JobSettings

    js = JobSettings()
    layer = js.apply_preset("Plywood 3mm (cut)")
    assert layer is not None
    assert layer.settings.speed_mm_s == 30.0
    assert layer.settings.power_pct == 90.0
    assert len(js.layers) == 1


def test_apply_unknown_preset():
    from snartlaser.job.settings import JobSettings

    js = JobSettings()
    result = js.apply_preset("Nonexistent Material XYZ")
    assert result is None
    assert len(js.layers) == 0


def test_enabled_layers():
    from snartlaser.job.layer import LayerList
    from snartlaser.core.types import CutSettings

    ll = LayerList()
    ll.add("Enabled 1")
    l2 = ll.add("Disabled")
    l2.settings.enabled = False
    ll.add("Enabled 2")

    enabled = ll.enabled_layers
    assert len(enabled) == 2
    assert all(l.enabled for l in enabled)
