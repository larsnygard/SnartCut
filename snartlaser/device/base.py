"""Abstract base class for all machine drivers.

Concrete drivers (GRBL, vinyl, Marlin …) must subclass :class:`BaseDevice`
and implement every abstract method.

Thread safety
~~~~~~~~~~~~~
:class:`BaseDevice` runs its I/O loop in a :class:`~PyQt6.QtCore.QThread`
sub-thread.  All public methods may be called from the main thread; they
communicate with the worker thread through a command queue.  Signals defined
here are emitted from the worker thread but PyQt6 automatically routes them
to the main thread (queued connection).
"""
from __future__ import annotations

import queue
import threading
from typing import List, Optional

from PyQt6.QtCore import QObject, QThread, pyqtSignal


class DeviceWorker(QThread):
    """Background thread that owns the serial connection and processes commands."""

    #: Raw response line received from the machine.
    line_received = pyqtSignal(str)
    #: Machine position update (x_mm, y_mm).
    position_update = pyqtSignal(float, float)
    #: Emitted after all queued commands have been sent.
    job_finished = pyqtSignal(bool)
    #: Status / error message.
    message = pyqtSignal(str)

    def __init__(self) -> None:
        super().__init__()
        self._cmd_queue: queue.Queue[Optional[str]] = queue.Queue()
        self._stop_event = threading.Event()

    def enqueue(self, cmd: str) -> None:
        self._cmd_queue.put(cmd)

    def enqueue_all(self, cmds: List[str]) -> None:
        for c in cmds:
            self._cmd_queue.put(c)
        self._cmd_queue.put(None)  # sentinel = job complete

    def stop(self) -> None:
        self._stop_event.set()
        self._cmd_queue.put(None)  # unblock get()

    def run(self) -> None:
        self._loop()

    def _loop(self) -> None:  # pragma: no cover
        """Main I/O loop; must be implemented by concrete workers."""
        raise NotImplementedError


class BaseDevice:
    """Abstract device interface.

    Concrete implementations must override :meth:`connect`, :meth:`disconnect`,
    :meth:`send_job`, :meth:`pause`, :meth:`resume` and :meth:`stop`.

    Note: Subclasses that also inherit from QObject must not mix ABC/ABCMeta
    with Qt's metaclass.  Use ``raise NotImplementedError`` in place of
    ``@abstractmethod`` to avoid metaclass conflicts.
    """

    def __init__(self) -> None:
        self._connected = False

    # ------------------------------------------------------------------
    # State queries
    # ------------------------------------------------------------------

    @property
    def is_connected(self) -> bool:
        return self._connected

    # ------------------------------------------------------------------
    # Interface (raise NotImplementedError instead of @abstractmethod
    # to avoid metaclass conflict with QObject)
    # ------------------------------------------------------------------

    def connect(self, port: str, baud_rate: int = 115200) -> bool:
        """Open the serial connection.  Returns ``True`` on success."""
        raise NotImplementedError

    def disconnect(self) -> None:
        """Close the serial connection."""
        raise NotImplementedError

    def send_job(self, lines: List[str]) -> None:
        """Queue *lines* for execution on the machine."""
        raise NotImplementedError

    def pause(self) -> None:
        """Pause the current job (feed-hold)."""
        raise NotImplementedError

    def resume(self) -> None:
        """Resume after a pause (cycle-start)."""
        raise NotImplementedError

    def stop(self) -> None:
        """Immediately stop the machine and clear the queue."""
        raise NotImplementedError

    def home(self) -> None:
        """Run the homing cycle."""
        raise NotImplementedError

    def jog(self, dx_mm: float, dy_mm: float, speed_mm_min: float = 3000) -> None:
        """Move the head by *dx_mm*, *dy_mm* at *speed_mm_min*."""
        raise NotImplementedError

    def get_status(self) -> str:
        """Return a one-line human-readable status string."""
        raise NotImplementedError

