"""Tests for snartlaser.core (config, event_bus, types)."""
from __future__ import annotations

import os
import tempfile

import pytest

# Ensure Qt does not look for a display
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")


def test_config_defaults():
    from snartlaser.core.config import Config

    cfg = Config()
    assert cfg.workspace_width_mm > 0
    assert cfg.workspace_height_mm > 0
    assert isinstance(cfg.show_grid, bool)
    assert cfg.device_baud_rate in (9600, 115200, 250000)


def test_config_roundtrip():
    from snartlaser.core.config import Config

    cfg = Config()
    cfg.workspace_width_mm = 300.0
    assert cfg.workspace_width_mm == 300.0

    cfg.last_directory = "/tmp/test"
    assert cfg.last_directory == "/tmp/test"


def test_config_recent_files():
    from snartlaser.core.config import Config

    cfg = Config()
    cfg.set("app/recent_files", [])  # reset
    cfg.add_recent_file("/a.slp")
    cfg.add_recent_file("/b.slp")
    cfg.add_recent_file("/a.slp")  # should move to front
    files = cfg.recent_files
    assert files[0] == "/a.slp"
    assert files.count("/a.slp") == 1


def test_cut_settings_serialisation():
    from snartlaser.core.types import AirAssist, CutSettings, LayerMode

    cs = CutSettings(
        name="Test",
        mode=LayerMode.FILL,
        speed_mm_s=200.0,
        power_pct=75.0,
        passes=2,
        air_assist=AirAssist.ON,
        color="#00ff00",
    )
    d = cs.to_dict()
    cs2 = CutSettings.from_dict(d)
    assert cs2.name == "Test"
    assert cs2.mode == LayerMode.FILL
    assert cs2.speed_mm_s == 200.0
    assert cs2.power_pct == 75.0
    assert cs2.passes == 2
    assert cs2.air_assist == AirAssist.ON
    assert cs2.color == "#00ff00"


def test_point_arithmetic():
    from snartlaser.core.types import Point

    a = Point(1.0, 2.0)
    b = Point(3.0, 4.0)
    c = a + b
    assert c.x == 4.0
    assert c.y == 6.0
    d = b - a
    assert d.x == 2.0
    assert d.y == 2.0


def test_bounding_box_center():
    from snartlaser.core.types import BoundingBox

    bb = BoundingBox(10.0, 20.0, 100.0, 50.0)
    assert bb.center.x == 60.0
    assert bb.center.y == 45.0
    assert bb.right == 110.0
    assert bb.bottom == 70.0


def test_event_bus_signals(qapp):
    from snartlaser.core.event_bus import EventBus

    bus = EventBus()
    received = []
    bus.file_opened.connect(lambda p: received.append(p))
    bus.file_opened.emit("/test/path.svg")
    assert received == ["/test/path.svg"]


def test_event_bus_device_connected(qapp):
    from snartlaser.core.event_bus import EventBus

    bus = EventBus()
    states = []
    bus.device_connected.connect(lambda v: states.append(v))
    bus.device_connected.emit(True)
    bus.device_connected.emit(False)
    assert states == [True, False]
