"""Vinyl cutter driver (HPGL over serial).

Implements :class:`~snartlaser.device.base.BaseDevice` for vinyl cutters
that accept HPGL commands over a serial port (e.g. Roland, Graphtec, Silhouette
with USB-serial adapter).

HPGL commands used
~~~~~~~~~~~~~~~~~~
* ``IN``   – initialise / reset
* ``SP<n>``– select pen (blade)
* ``PU<x>,<y>`` – pen up + move
* ``PD<x>,<y>`` – pen down + move (cut)
* ``PA<x>,<y>`` – pen absolute move (pen state unchanged)
* ``VS<n>`` – velocity / cutting speed (1–120 cm/s, machine-dependent)
* ``FS<n>`` – force setting (grams, machine-dependent)

Plotter units: 1 unit = 1/40 mm (0.025 mm).
"""
from __future__ import annotations

import queue
import threading
import time
from typing import List, Optional

import serial
from PyQt6.QtCore import QObject, QThread, pyqtSignal

from snartlaser.device.base import BaseDevice


class _HpglWorker(QThread):
    message = pyqtSignal(str)
    job_finished = pyqtSignal(bool)

    def __init__(self, port: str, baud_rate: int) -> None:
        super().__init__()
        self._port = port
        self._baud = baud_rate
        self._ser: Optional[serial.Serial] = None
        self._cmd_queue: queue.Queue[Optional[str]] = queue.Queue()
        self._stop = threading.Event()
        self._connected = False

    def open(self) -> bool:
        try:
            self._ser = serial.Serial(
                self._port,
                self._baud,
                timeout=1.0,
                write_timeout=1.0,
                # Many vinyl cutters use RTS/CTS flow control
                rtscts=True,
            )
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

    def enqueue_all(self, cmds: List[str]) -> None:
        for c in cmds:
            self._cmd_queue.put(c)
        self._cmd_queue.put(None)

    def run(self) -> None:
        if not self._connected:
            return
        job_running = False
        success = True

        while not self._stop.is_set():
            try:
                cmd = self._cmd_queue.get(timeout=0.1)
            except queue.Empty:
                continue

            if cmd is None:
                if job_running:
                    job_running = False
                    self.job_finished.emit(success)
                continue

            job_running = True
            try:
                if self._ser and self._ser.is_open:
                    # HPGL lines are terminated with semicolon or newline;
                    # we ensure semicolons are already in the command strings.
                    data = (cmd.strip() + "\r\n").encode("ascii", errors="replace")
                    self._ser.write(data)
                    # Small delay to avoid overrunning the cutter's buffer
                    time.sleep(0.005)
            except Exception as exc:
                self.message.emit(f"Send error: {exc}")
                success = False


class VinylDevice(BaseDevice, QObject):
    """HPGL vinyl cutter driver.

    Args:
        blade_force: Cutting force in grams (machine-dependent default: 80 g).
        cutting_speed: Cutting speed in cm/s (machine-dependent default: 10).
    """

    job_finished = pyqtSignal(bool)
    message = pyqtSignal(str)

    def __init__(
        self,
        parent: Optional[QObject] = None,
        blade_force: int = 80,
        cutting_speed: int = 10,
    ) -> None:
        BaseDevice.__init__(self)
        QObject.__init__(self, parent)
        self._worker: Optional[_HpglWorker] = None
        self.blade_force = blade_force
        self.cutting_speed = cutting_speed

    # ------------------------------------------------------------------
    # BaseDevice interface
    # ------------------------------------------------------------------

    def connect(self, port: str, baud_rate: int = 9600) -> bool:
        self._worker = _HpglWorker(port, baud_rate)
        self._worker.message.connect(self.message)
        self._worker.job_finished.connect(self.job_finished)

        if not self._worker.open():
            self._worker = None
            return False

        self._worker.start()
        self._connected = True
        return True

    def disconnect(self) -> None:
        if self._worker:
            self._worker.close()
            self._worker.wait(3000)
            self._worker = None
        self._connected = False

    def send_job(self, lines: List[str]) -> None:
        if self._worker and self._connected:
            # Prepend speed and force settings
            preamble = [
                f"VS{self.cutting_speed};",
                f"FS{self.blade_force};",
            ]
            self._worker.enqueue_all(preamble + lines)

    def pause(self) -> None:
        # Most HPGL cutters do not support mid-job pause over serial
        self.message.emit("Pause not supported on this device")

    def resume(self) -> None:
        self.message.emit("Resume not supported on this device")

    def stop(self) -> None:
        if self._worker and self._worker._ser and self._worker._ser.is_open:
            try:
                self._worker._ser.write(b"IN;\r\n")  # reset
            except Exception:
                pass

    def home(self) -> None:
        if self._worker:
            self._worker.enqueue_all(["PU0,0;"])

    def jog(self, dx_mm: float, dy_mm: float, speed_mm_min: float = 3000) -> None:
        if self._worker and self._connected:
            _U = 40.0  # HPGL units per mm
            hx = int(dx_mm * _U)
            hy = int(dy_mm * _U)
            self._worker.enqueue_all([f"PR{hx},{hy};"])

    def get_status(self) -> str:
        return "Connected" if self._connected else "Disconnected"
