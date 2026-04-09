"""GRBL-based laser cutter driver.

Implements :class:`~snartlaser.device.base.BaseDevice` for machines running
`GRBL <https://github.com/gnea/grbl>`_ firmware.

Protocol summary
~~~~~~~~~~~~~~~~
1. After connection GRBL sends ``Grbl <ver> ...``.
2. Commands are sent one line at a time.
3. GRBL responds with ``ok`` or ``error:<N>`` for each command.
4. A ``?`` query returns the real-time status report.
5. ``!`` = feed-hold, ``~`` = cycle-start, ``\x18`` = soft-reset.

This driver runs a :class:`~snartlaser.device.base.DeviceWorker` in a background
thread and exposes Qt signals for use in the UI.
"""
from __future__ import annotations

import queue
import re
import threading
import time
from typing import List, Optional

import serial
from PyQt6.QtCore import QObject, QThread, pyqtSignal

from snartlaser.device.base import BaseDevice


class _GrblWorker(QThread):
    """Background thread that owns the serial port and processes the send queue."""

    line_received = pyqtSignal(str)
    position_update = pyqtSignal(float, float)
    job_finished = pyqtSignal(bool)
    message = pyqtSignal(str)

    def __init__(self, port: str, baud_rate: int) -> None:
        super().__init__()
        self._port = port
        self._baud = baud_rate
        self._ser: Optional[serial.Serial] = None
        self._cmd_queue: queue.Queue[Optional[str]] = queue.Queue()
        self._stop = threading.Event()
        self._connected = False

    # ------------------------------------------------------------------
    # Control
    # ------------------------------------------------------------------

    def open(self) -> bool:
        try:
            self._ser = serial.Serial(
                self._port,
                self._baud,
                timeout=1.0,
                write_timeout=1.0,
            )
            time.sleep(2)  # GRBL initialisation time
            self._connected = True
            return True
        except serial.SerialException as exc:
            self.message.emit(f"Connection error: {exc}")
            return False

    def close(self) -> None:
        self._stop.set()
        self._cmd_queue.put(None)
        if self._ser and self._ser.is_open:
            try:
                self._ser.close()
            except Exception:
                pass
        self._connected = False

    def enqueue(self, cmd: str) -> None:
        self._cmd_queue.put(cmd)

    def enqueue_all(self, cmds: List[str]) -> None:
        for c in cmds:
            self._cmd_queue.put(c)
        self._cmd_queue.put(None)  # sentinel

    def send_immediate(self, byte: bytes) -> None:
        if self._ser and self._ser.is_open:
            self._ser.write(byte)

    # ------------------------------------------------------------------
    # Thread entry
    # ------------------------------------------------------------------

    def run(self) -> None:
        if not self._connected:
            return
        self.message.emit("GRBL connected")
        job_running = False
        success = True

        while not self._stop.is_set():
            # Read incoming data
            if self._ser and self._ser.in_waiting:
                try:
                    raw = self._ser.readline().decode(errors="replace").strip()
                    if raw:
                        self.line_received.emit(raw)
                        self._parse_response(raw)
                except Exception as exc:
                    self.message.emit(f"Read error: {exc}")

            # Send next command from queue (non-blocking)
            try:
                cmd = self._cmd_queue.get_nowait()
            except queue.Empty:
                time.sleep(0.01)
                continue

            if cmd is None:
                # Sentinel: job complete or stop requested
                if job_running:
                    job_running = False
                    self.job_finished.emit(success)
                continue

            if not job_running:
                job_running = True

            try:
                if self._ser and self._ser.is_open:
                    line = (cmd.strip() + "\n").encode()
                    self._ser.write(line)
                    # Simple wait-for-ok handshake
                    ok = self._wait_for_ok(timeout=10.0)
                    if not ok:
                        self.message.emit(f"No OK for: {cmd}")
                        success = False
            except Exception as exc:
                self.message.emit(f"Send error: {exc}")
                success = False

    def _wait_for_ok(self, timeout: float = 10.0) -> bool:
        if not self._ser:
            return False
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self._ser.in_waiting:
                try:
                    resp = self._ser.readline().decode(errors="replace").strip()
                    self.line_received.emit(resp)
                    if resp.lower().startswith("ok"):
                        return True
                    if resp.lower().startswith("error"):
                        self.message.emit(f"GRBL error: {resp}")
                        return False
                except Exception:
                    return False
            time.sleep(0.005)
        return False

    _STATUS_RE = re.compile(r"<\w+\|MPos:([\d.\-]+),([\d.\-]+),[\d.\-]+")

    def _parse_response(self, line: str) -> None:
        m = self._STATUS_RE.search(line)
        if m:
            try:
                x = float(m.group(1))
                y = float(m.group(2))
                self.position_update.emit(x, y)
            except ValueError:
                pass


class GrblDevice(BaseDevice, QObject):
    """GRBL laser cutter driver.

    Args:
        parent: Optional :class:`QObject` parent.
    """

    # Qt signals (re-emitted from the worker)
    line_received = pyqtSignal(str)
    position_update = pyqtSignal(float, float)
    job_finished = pyqtSignal(bool)
    message = pyqtSignal(str)

    def __init__(self, parent: Optional[QObject] = None) -> None:
        BaseDevice.__init__(self)
        QObject.__init__(self, parent)
        self._worker: Optional[_GrblWorker] = None
        self._status = "Disconnected"

    # ------------------------------------------------------------------
    # BaseDevice interface
    # ------------------------------------------------------------------

    def connect(self, port: str, baud_rate: int = 115200) -> bool:
        self._worker = _GrblWorker(port, baud_rate)
        # Forward worker signals
        self._worker.line_received.connect(self.line_received)
        self._worker.position_update.connect(self.position_update)
        self._worker.job_finished.connect(self.job_finished)
        self._worker.message.connect(self.message)

        if not self._worker.open():
            self._worker = None
            return False

        self._worker.start()
        self._connected = True
        self._status = "Idle"
        return True

    def disconnect(self) -> None:
        if self._worker:
            self._worker.close()
            self._worker.wait(3000)
            self._worker = None
        self._connected = False
        self._status = "Disconnected"

    def send_job(self, lines: List[str]) -> None:
        if self._worker and self._connected:
            self._worker.enqueue_all(lines)

    def pause(self) -> None:
        if self._worker:
            self._worker.send_immediate(b"!")

    def resume(self) -> None:
        if self._worker:
            self._worker.send_immediate(b"~")

    def stop(self) -> None:
        if self._worker:
            self._worker.send_immediate(b"\x18")  # GRBL soft-reset

    def home(self) -> None:
        if self._worker:
            self._worker.enqueue("$H")

    def jog(self, dx_mm: float, dy_mm: float, speed_mm_min: float = 3000) -> None:
        if self._worker and self._connected:
            self._worker.enqueue(
                f"$J=G91 G21 X{dx_mm:.3f} Y{dy_mm:.3f} F{speed_mm_min:.0f}"
            )

    def get_status(self) -> str:
        return self._status

    def query_status(self) -> None:
        """Send a real-time status query ``?`` to GRBL."""
        if self._worker:
            self._worker.send_immediate(b"?")
