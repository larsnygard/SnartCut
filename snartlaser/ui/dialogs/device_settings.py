"""Device settings dialog.

Lets the user pick the serial port, baud rate and device type.
"""
from __future__ import annotations

from typing import Optional

from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import (
    QComboBox,
    QDialog,
    QDialogButtonBox,
    QFormLayout,
    QLabel,
    QVBoxLayout,
    QWidget,
)

from snartlaser.core.config import Config
from snartlaser.core.types import DeviceType


class DeviceSettingsDialog(QDialog):
    """Modal dialog for configuring the machine connection."""

    def __init__(self, config: Config, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self.config = config
        self.setWindowTitle("Device Settings")
        self.setMinimumWidth(380)
        self._build_ui()

    def _build_ui(self) -> None:
        layout = QVBoxLayout(self)

        form = QFormLayout()

        # Device type
        self._device_type = QComboBox()
        for dt in DeviceType:
            self._device_type.addItem(dt.value.replace("_", " ").title(), dt.value)
        idx = self._device_type.findData(self.config.device_type)
        if idx >= 0:
            self._device_type.setCurrentIndex(idx)
        form.addRow("Device type:", self._device_type)

        # Serial port
        self._port = QComboBox()
        self._port.setEditable(True)
        self._populate_ports()
        self._port.setCurrentText(self.config.device_port)
        form.addRow("Serial port:", self._port)

        # Baud rate
        self._baud = QComboBox()
        for b in [9600, 19200, 38400, 57600, 115200, 250000]:
            self._baud.addItem(str(b), b)
        idx = self._baud.findData(self.config.device_baud_rate)
        if idx >= 0:
            self._baud.setCurrentIndex(idx)
        form.addRow("Baud rate:", self._baud)

        layout.addLayout(form)

        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        buttons.accepted.connect(self._save)
        buttons.rejected.connect(self.reject)
        layout.addWidget(buttons)

    def _populate_ports(self) -> None:
        """List available serial ports."""
        try:
            import serial.tools.list_ports  # type: ignore
            ports = serial.tools.list_ports.comports()
            for p in ports:
                self._port.addItem(p.device)
        except Exception:
            pass
        # Always provide manual entry items
        for fallback in ["/dev/ttyUSB0", "/dev/ttyACM0", "COM3"]:
            if self._port.findText(fallback) < 0:
                self._port.addItem(fallback)

    def _save(self) -> None:
        self.config.device_port = self._port.currentText()
        self.config.device_baud_rate = int(self._baud.currentData())
        self.config.device_type = self._device_type.currentData()
        self.accept()
