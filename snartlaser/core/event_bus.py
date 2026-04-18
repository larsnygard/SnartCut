"""Application-wide event bus using Qt signals.

All major application events are declared here so that modules can
communicate without hard dependencies on each other.

Usage::

    bus = EventBus()

    # Connect a handler
    bus.file_opened.connect(lambda path: print("Opened:", path))

    # Emit an event
    bus.file_opened.emit("/home/user/design.svg")
"""
from __future__ import annotations

from PyQt6.QtCore import QObject, pyqtSignal


class EventBus(QObject):
    """Central event hub.  All signals live here as class attributes."""

    # ------------------------------------------------------------------
    # File / project events
    # ------------------------------------------------------------------

    #: Emitted after a file has been successfully opened.  Carries the path.
    file_opened = pyqtSignal(str)

    #: Emitted after the project has been saved.  Carries the path.
    file_saved = pyqtSignal(str)

    #: Emitted when the project's modified state changes.
    project_modified = pyqtSignal(bool)

    # ------------------------------------------------------------------
    # Canvas events
    # ------------------------------------------------------------------

    #: Emitted when the canvas selection changes.  Carries list of item IDs.
    selection_changed = pyqtSignal(list)

    #: Emitted when the active drawing tool changes.  Carries tool name.
    tool_changed = pyqtSignal(str)

    #: Emitted when the zoom level changes.  Carries the new zoom factor.
    zoom_changed = pyqtSignal(float)

    # ------------------------------------------------------------------
    # Layer / job events
    # ------------------------------------------------------------------

    #: Emitted when the layer list changes.
    layers_changed = pyqtSignal()

    #: Emitted when a layer's settings change.  Carries layer index.
    layer_updated = pyqtSignal(int)

    # ------------------------------------------------------------------
    # Device events
    # ------------------------------------------------------------------

    #: Emitted when the device connection state changes.
    device_connected = pyqtSignal(bool)

    #: Emitted during a job run with progress 0–100.
    job_progress = pyqtSignal(int)

    #: Emitted when the machine position changes (x_mm, y_mm).
    position_changed = pyqtSignal(float, float)

    #: Emitted with a status/error message from the device.
    device_message = pyqtSignal(str)

    #: Emitted when a job finishes (success=True) or fails (success=False).
    job_finished = pyqtSignal(bool)
