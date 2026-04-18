"""Design view – :class:`QGraphicsView` wrapper with zoom and pan support."""
from __future__ import annotations

import math

from PyQt6.QtCore import Qt, pyqtSignal
from PyQt6.QtGui import QWheelEvent, QMouseEvent, QKeyEvent
from PyQt6.QtWidgets import QGraphicsView

from snartlaser.canvas.scene import DesignScene


class DesignView(QGraphicsView):
    """Scrollable, zoomable view for :class:`DesignScene`.

    * Mouse wheel → zoom in/out (Ctrl optional)
    * Middle-button drag (or Space+drag) → pan
    * Ctrl+= / Ctrl+- / Ctrl+0 → keyboard zoom
    """

    zoom_changed = pyqtSignal(float)

    _ZOOM_FACTOR = 1.15
    _ZOOM_MIN = 0.01
    _ZOOM_MAX = 100.0

    def __init__(self, scene: DesignScene, parent=None) -> None:
        super().__init__(scene, parent)

        self.setRenderHint(self.renderHints().__class__.Antialiasing, True)
        self.setRenderHint(
            self.renderHints().__class__.SmoothPixmapTransform, True
        )
        self.setDragMode(QGraphicsView.DragMode.RubberBandDrag)
        self.setTransformationAnchor(
            QGraphicsView.ViewportAnchor.AnchorUnderMouse
        )
        self.setResizeAnchor(QGraphicsView.ViewportAnchor.AnchorViewCenter)

        self._current_zoom: float = 1.0
        self._panning = False
        self._pan_start_x = 0
        self._pan_start_y = 0

    # ------------------------------------------------------------------
    # Zoom
    # ------------------------------------------------------------------

    @property
    def current_zoom(self) -> float:
        return self._current_zoom

    def zoom_to(self, factor: float) -> None:
        """Set the absolute zoom *factor* (1.0 = 100 %)."""
        factor = max(self._ZOOM_MIN, min(self._ZOOM_MAX, factor))
        scale = factor / self._current_zoom
        self.scale(scale, scale)
        self._current_zoom = factor
        self.zoom_changed.emit(factor)

    def zoom_fit(self) -> None:
        """Fit the workspace into the viewport."""
        self.fitInView(self.scene().sceneRect(), Qt.AspectRatioMode.KeepAspectRatio)
        t = self.transform()
        self._current_zoom = t.m11()
        self.zoom_changed.emit(self._current_zoom)

    def zoom_in(self) -> None:
        self.zoom_to(self._current_zoom * self._ZOOM_FACTOR)

    def zoom_out(self) -> None:
        self.zoom_to(self._current_zoom / self._ZOOM_FACTOR)

    # ------------------------------------------------------------------
    # Event overrides
    # ------------------------------------------------------------------

    def wheelEvent(self, event: QWheelEvent) -> None:
        delta = event.angleDelta().y()
        if delta > 0:
            self.zoom_in()
        else:
            self.zoom_out()

    def mousePressEvent(self, event: QMouseEvent) -> None:
        if event.button() == Qt.MouseButton.MiddleButton:
            self._panning = True
            self._pan_start_x = event.position().x()
            self._pan_start_y = event.position().y()
            self.setCursor(Qt.CursorShape.ClosedHandCursor)
            event.accept()
            return
        super().mousePressEvent(event)

    def mouseMoveEvent(self, event: QMouseEvent) -> None:
        if self._panning:
            dx = event.position().x() - self._pan_start_x
            dy = event.position().y() - self._pan_start_y
            self._pan_start_x = event.position().x()
            self._pan_start_y = event.position().y()
            self.horizontalScrollBar().setValue(
                int(self.horizontalScrollBar().value() - dx)
            )
            self.verticalScrollBar().setValue(
                int(self.verticalScrollBar().value() - dy)
            )
            event.accept()
            return
        super().mouseMoveEvent(event)

    def mouseReleaseEvent(self, event: QMouseEvent) -> None:
        if event.button() == Qt.MouseButton.MiddleButton:
            self._panning = False
            self.setCursor(Qt.CursorShape.ArrowCursor)
            event.accept()
            return
        super().mouseReleaseEvent(event)

    def keyPressEvent(self, event: QKeyEvent) -> None:
        if event.modifiers() & Qt.KeyboardModifier.ControlModifier:
            if event.key() == Qt.Key.Key_Equal:
                self.zoom_in()
                return
            if event.key() == Qt.Key.Key_Minus:
                self.zoom_out()
                return
            if event.key() == Qt.Key.Key_0:
                self.zoom_fit()
                return
        super().keyPressEvent(event)
