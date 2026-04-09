"""Top-level :class:`Application` class.

Wraps :class:`QApplication` and wires together the major subsystems:

* :class:`~snartlaser.core.config.Config`  – persistent settings
* :class:`~snartlaser.core.event_bus.EventBus` – application-wide signals
* :class:`~snartlaser.ui.mainwindow.MainWindow` – the main UI window
"""
from __future__ import annotations

import sys
from typing import List

from PyQt6.QtWidgets import QApplication

from snartlaser.core.config import Config
from snartlaser.core.event_bus import EventBus


class Application(QApplication):
    """Subclass of :class:`QApplication` that initialises all subsystems."""

    def __init__(self, argv: List[str]) -> None:
        super().__init__(argv)

        self.setApplicationName("SnartLaser")
        self.setApplicationVersion("0.1.0")
        self.setOrganizationName("SnartLaser")
        self.setOrganizationDomain("snartlaser.io")

        # Core subsystems
        self.config = Config()
        self.event_bus = EventBus()

        # Deferred UI import so that the module can be imported without a display
        from snartlaser.ui.mainwindow import MainWindow

        self._main_window = MainWindow(self.config, self.event_bus)
        self._main_window.show()
