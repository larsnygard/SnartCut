"""Device panel – connection control and status display."""
from __future__ import annotations

from typing import Optional

from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtWidgets import (
    QGroupBox,
    QHBoxLayout,
    QLabel,
    QProgressBar,
    QPushButton,
    QTextEdit,
    QVBoxLayout,
    QWidget,
)

from snartlaser.core.config import Config
from snartlaser.core.event_bus import EventBus
from snartlaser.core.types import DeviceType
from snartlaser.device.grbl import GrblDevice
from snartlaser.device.vinyl import VinylDevice


class DevicePanel(QWidget):
    """Sidebar panel for machine connection, jogging and job control."""

    def __init__(
        self,
        config: Config,
        event_bus: EventBus,
        parent: Optional[QWidget] = None,
    ) -> None:
        super().__init__(parent)
        self.config = config
        self.event_bus = event_bus
        self._device: Optional[GrblDevice | VinylDevice] = None
        self._build_ui()
        self._connect_signals()

        # Poll GRBL status every second
        self._status_timer = QTimer(self)
        self._status_timer.setInterval(1000)
        self._status_timer.timeout.connect(self._poll_status)

    # ------------------------------------------------------------------
    # UI construction
    # ------------------------------------------------------------------

    def _build_ui(self) -> None:
        layout = QVBoxLayout(self)
        layout.setContentsMargins(4, 4, 4, 4)

        # Connection group
        conn_group = QGroupBox("Connection")
        conn_layout = QVBoxLayout(conn_group)

        row1 = QHBoxLayout()
        self._status_label = QLabel("Disconnected")
        self._status_label.setStyleSheet("color: #e74c3c;")
        row1.addWidget(self._status_label)
        row1.addStretch()
        conn_layout.addLayout(row1)

        row2 = QHBoxLayout()
        self._connect_btn = QPushButton("Connect")
        self._connect_btn.clicked.connect(self._toggle_connection)
        row2.addWidget(self._connect_btn)

        self._settings_btn = QPushButton("Settings…")
        self._settings_btn.clicked.connect(self._open_settings)
        row2.addWidget(self._settings_btn)
        conn_layout.addLayout(row2)

        layout.addWidget(conn_group)

        # Position group
        pos_group = QGroupBox("Position")
        pos_layout = QVBoxLayout(pos_group)
        self._pos_label = QLabel("X: 0.000  Y: 0.000")
        pos_layout.addWidget(self._pos_label)

        # Jog controls
        jog_grid = QHBoxLayout()
        for label, dx, dy in [("←", -10, 0), ("↑", 0, -10), ("→", 10, 0), ("↓", 0, 10)]:
            btn = QPushButton(label)
            btn.setFixedSize(36, 36)
            btn.clicked.connect(lambda _, x=dx, y=dy: self._jog(x, y))
            jog_grid.addWidget(btn)
        pos_layout.addLayout(jog_grid)

        self._home_btn = QPushButton("Home")
        self._home_btn.clicked.connect(self._home)
        pos_layout.addWidget(self._home_btn)

        layout.addWidget(pos_group)

        # Job control
        job_group = QGroupBox("Job Control")
        job_layout = QVBoxLayout(job_group)

        self._progress = QProgressBar()
        self._progress.setRange(0, 100)
        self._progress.setValue(0)
        job_layout.addWidget(self._progress)

        btns = QHBoxLayout()
        self._run_btn = QPushButton("▶ Run")
        self._run_btn.setEnabled(False)
        self._run_btn.clicked.connect(self._run_job)
        btns.addWidget(self._run_btn)

        self._pause_btn = QPushButton("⏸")
        self._pause_btn.setEnabled(False)
        self._pause_btn.clicked.connect(self._pause)
        btns.addWidget(self._pause_btn)

        self._stop_btn = QPushButton("⏹")
        self._stop_btn.setEnabled(False)
        self._stop_btn.clicked.connect(self._stop)
        btns.addWidget(self._stop_btn)

        job_layout.addLayout(btns)
        layout.addWidget(job_group)

        # Console
        console_group = QGroupBox("Console")
        console_layout = QVBoxLayout(console_group)
        self._console = QTextEdit()
        self._console.setReadOnly(True)
        self._console.setMaximumHeight(120)
        self._console.setStyleSheet("font-family: monospace; font-size: 11px;")
        console_layout.addWidget(self._console)
        layout.addWidget(console_group)

        layout.addStretch()

    def _connect_signals(self) -> None:
        self.event_bus.device_connected.connect(self._on_device_connected)
        self.event_bus.job_progress.connect(self._progress.setValue)
        self.event_bus.position_changed.connect(self._on_position)
        self.event_bus.device_message.connect(self._log)
        self.event_bus.job_finished.connect(self._on_job_finished)

    # ------------------------------------------------------------------
    # Slots
    # ------------------------------------------------------------------

    def _toggle_connection(self) -> None:
        if self._device and self._device.is_connected:
            self._disconnect()
        else:
            self._do_connect()

    def _do_connect(self) -> None:
        port = self.config.device_port
        baud = self.config.device_baud_rate
        dtype = self.config.device_type

        if dtype == DeviceType.VINYL_CUTTER.value:
            self._device = VinylDevice()
            self._device.message.connect(self._log)
            self._device.job_finished.connect(
                lambda ok: self.event_bus.job_finished.emit(ok)
            )
        else:
            self._device = GrblDevice()
            self._device.message.connect(self._log)
            self._device.position_update.connect(
                lambda x, y: self.event_bus.position_changed.emit(x, y)
            )
            self._device.job_finished.connect(
                lambda ok: self.event_bus.job_finished.emit(ok)
            )

        ok = self._device.connect(port, baud)
        self.event_bus.device_connected.emit(ok)
        if ok:
            self._log(f"Connected to {port} @ {baud}")
            if isinstance(self._device, GrblDevice):
                self._status_timer.start()
        else:
            self._log(f"Failed to connect to {port}")

    def _disconnect(self) -> None:
        if self._device:
            self._status_timer.stop()
            self._device.disconnect()
            self._device = None
        self.event_bus.device_connected.emit(False)

    def _on_device_connected(self, connected: bool) -> None:
        if connected:
            self._status_label.setText("Connected")
            self._status_label.setStyleSheet("color: #2ecc71;")
            self._connect_btn.setText("Disconnect")
            self._run_btn.setEnabled(True)
            self._pause_btn.setEnabled(True)
            self._stop_btn.setEnabled(True)
        else:
            self._status_label.setText("Disconnected")
            self._status_label.setStyleSheet("color: #e74c3c;")
            self._connect_btn.setText("Connect")
            self._run_btn.setEnabled(False)
            self._pause_btn.setEnabled(False)
            self._stop_btn.setEnabled(False)

    def _on_position(self, x: float, y: float) -> None:
        self._pos_label.setText(f"X: {x:.3f}  Y: {y:.3f}")

    def _on_job_finished(self, success: bool) -> None:
        self._progress.setValue(100 if success else 0)
        self._log("Job finished" if success else "Job failed/stopped")

    def _log(self, msg: str) -> None:
        self._console.append(msg)
        sb = self._console.verticalScrollBar()
        sb.setValue(sb.maximum())

    def _jog(self, dx: float, dy: float) -> None:
        if self._device and self._device.is_connected:
            self._device.jog(dx, dy)

    def _home(self) -> None:
        if self._device and self._device.is_connected:
            self._device.home()

    def _pause(self) -> None:
        if self._device:
            self._device.pause()

    def _stop(self) -> None:
        if self._device:
            self._device.stop()

    def _run_job(self) -> None:
        """Emit a signal that the main window handles to generate & send G-code."""
        self.event_bus.job_progress.emit(0)
        # Actual job dispatch is handled by MainWindow._run_job
        from PyQt6.QtCore import QMetaObject, Qt
        # We use a custom signal approach via event_bus
        self._log("Requesting job start…")

    def _open_settings(self) -> None:
        from snartlaser.ui.dialogs.device_settings import DeviceSettingsDialog

        dlg = DeviceSettingsDialog(self.config, self)
        dlg.exec()

    def _poll_status(self) -> None:
        if isinstance(self._device, GrblDevice) and self._device.is_connected:
            self._device.query_status()
