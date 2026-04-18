"""Interactive drawing tools for the canvas.

Each tool subclasses :class:`BaseTool` and responds to mouse events
forwarded by :class:`~snartlaser.canvas.scene.DesignScene`.  Tools are
stateless between mouse events (state is stored on the tool object) so
they can be swapped without losing scene state.
"""
from __future__ import annotations

from typing import Optional

from PyQt6.QtCore import QPointF, Qt
from PyQt6.QtGui import QPainterPath, QPen, QColor, QBrush
from PyQt6.QtWidgets import QGraphicsEllipseItem, QGraphicsLineItem, QGraphicsRectItem

from snartlaser.canvas.items import DesignItem
from snartlaser.core.types import ToolType


class BaseTool:
    """Abstract base for canvas tools."""

    tool_type: ToolType = ToolType.SELECT

    def __init__(self, scene: "DesignScene") -> None:  # type: ignore[name-defined]
        self.scene = scene
        self._active_color: str = "#ff0000"

    def set_color(self, color: str) -> None:
        self._active_color = color

    def mouse_press(self, pos: QPointF, button: Qt.MouseButton) -> None:
        pass

    def mouse_move(self, pos: QPointF) -> None:
        pass

    def mouse_release(self, pos: QPointF, button: Qt.MouseButton) -> None:
        pass

    def key_press(self, key: int) -> None:
        pass

    def activate(self) -> None:
        pass

    def deactivate(self) -> None:
        pass


class SelectTool(BaseTool):
    """Selection / move tool."""

    tool_type = ToolType.SELECT

    def mouse_press(self, pos: QPointF, button: Qt.MouseButton) -> None:
        # Selection is handled natively by the scene / view
        pass


class PanTool(BaseTool):
    """Canvas panning tool."""

    tool_type = ToolType.PAN

    def activate(self) -> None:
        from PyQt6.QtCore import Qt
        self.scene.views()[0].setDragMode(
            self.scene.views()[0].DragMode.ScrollHandDrag
        )

    def deactivate(self) -> None:
        self.scene.views()[0].setDragMode(
            self.scene.views()[0].DragMode.RubberBandDrag
        )


class RectangleTool(BaseTool):
    """Draw rectangles by click-and-drag."""

    tool_type = ToolType.RECTANGLE

    def __init__(self, scene: "DesignScene") -> None:  # type: ignore[name-defined]
        super().__init__(scene)
        self._start: Optional[QPointF] = None
        self._preview: Optional[QGraphicsRectItem] = None

    def mouse_press(self, pos: QPointF, button: Qt.MouseButton) -> None:
        if button == Qt.MouseButton.LeftButton:
            self._start = pos
            self._preview = QGraphicsRectItem(pos.x(), pos.y(), 0, 0)
            pen = QPen(QColor(self._active_color))
            pen.setWidthF(0.3)
            self._preview.setPen(pen)
            self._preview.setBrush(QBrush(Qt.BrushStyle.NoBrush))
            self.scene.addItem(self._preview)

    def mouse_move(self, pos: QPointF) -> None:
        if self._start and self._preview:
            x = min(self._start.x(), pos.x())
            y = min(self._start.y(), pos.y())
            w = abs(pos.x() - self._start.x())
            h = abs(pos.y() - self._start.y())
            self._preview.setRect(x, y, w, h)

    def mouse_release(self, pos: QPointF, button: Qt.MouseButton) -> None:
        if button == Qt.MouseButton.LeftButton and self._start and self._preview:
            r = self._preview.rect()
            if r.width() > 0.1 and r.height() > 0.1:
                path = QPainterPath()
                path.addRect(r)
                item = DesignItem(path, self._active_color)
                self.scene.addItem(item)
                self.scene.item_added.emit(item.item_id)
            self.scene.removeItem(self._preview)
            self._preview = None
            self._start = None


class EllipseTool(BaseTool):
    """Draw ellipses by click-and-drag."""

    tool_type = ToolType.ELLIPSE

    def __init__(self, scene: "DesignScene") -> None:  # type: ignore[name-defined]
        super().__init__(scene)
        self._start: Optional[QPointF] = None
        self._preview: Optional[QGraphicsEllipseItem] = None

    def mouse_press(self, pos: QPointF, button: Qt.MouseButton) -> None:
        if button == Qt.MouseButton.LeftButton:
            self._start = pos
            self._preview = QGraphicsEllipseItem(pos.x(), pos.y(), 0, 0)
            pen = QPen(QColor(self._active_color))
            pen.setWidthF(0.3)
            self._preview.setPen(pen)
            self._preview.setBrush(QBrush(Qt.BrushStyle.NoBrush))
            self.scene.addItem(self._preview)

    def mouse_move(self, pos: QPointF) -> None:
        if self._start and self._preview:
            x = min(self._start.x(), pos.x())
            y = min(self._start.y(), pos.y())
            w = abs(pos.x() - self._start.x())
            h = abs(pos.y() - self._start.y())
            self._preview.setRect(x, y, w, h)

    def mouse_release(self, pos: QPointF, button: Qt.MouseButton) -> None:
        if button == Qt.MouseButton.LeftButton and self._start and self._preview:
            r = self._preview.rect()
            if r.width() > 0.1 and r.height() > 0.1:
                path = QPainterPath()
                path.addEllipse(r)
                item = DesignItem(path, self._active_color)
                self.scene.addItem(item)
                self.scene.item_added.emit(item.item_id)
            self.scene.removeItem(self._preview)
            self._preview = None
            self._start = None


class LineTool(BaseTool):
    """Draw straight lines."""

    tool_type = ToolType.LINE

    def __init__(self, scene: "DesignScene") -> None:  # type: ignore[name-defined]
        super().__init__(scene)
        self._start: Optional[QPointF] = None
        self._preview: Optional[QGraphicsLineItem] = None

    def mouse_press(self, pos: QPointF, button: Qt.MouseButton) -> None:
        if button == Qt.MouseButton.LeftButton:
            self._start = pos
            self._preview = QGraphicsLineItem(pos.x(), pos.y(), pos.x(), pos.y())
            pen = QPen(QColor(self._active_color))
            pen.setWidthF(0.3)
            self._preview.setPen(pen)
            self.scene.addItem(self._preview)

    def mouse_move(self, pos: QPointF) -> None:
        if self._start and self._preview:
            line = self._preview.line()
            self._preview.setLine(self._start.x(), self._start.y(), pos.x(), pos.y())

    def mouse_release(self, pos: QPointF, button: Qt.MouseButton) -> None:
        if button == Qt.MouseButton.LeftButton and self._start and self._preview:
            path = QPainterPath()
            path.moveTo(self._start)
            path.lineTo(pos)
            if path.length() > 0.1:
                item = DesignItem(path, self._active_color)
                self.scene.addItem(item)
                self.scene.item_added.emit(item.item_id)
            self.scene.removeItem(self._preview)
            self._preview = None
            self._start = None


# Registry mapping ToolType → class
TOOL_REGISTRY = {
    ToolType.SELECT: SelectTool,
    ToolType.PAN: PanTool,
    ToolType.RECTANGLE: RectangleTool,
    ToolType.ELLIPSE: EllipseTool,
    ToolType.LINE: LineTool,
}
