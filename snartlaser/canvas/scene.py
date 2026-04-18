"""Design scene – the central :class:`QGraphicsScene` subclass.

The scene holds all :class:`~snartlaser.canvas.items.DesignItem` objects,
draws the workspace border and grid, and dispatches mouse events to the
active :class:`~snartlaser.canvas.tools.BaseTool`.
"""
from __future__ import annotations

import json
from typing import Dict, List, Optional, Tuple

from PyQt6.QtCore import QLineF, QPointF, QRectF, Qt, pyqtSignal
from PyQt6.QtGui import (
    QBrush,
    QColor,
    QPainter,
    QPen,
    QPainterPath,
)
from PyQt6.QtWidgets import QGraphicsScene, QGraphicsItem

from snartlaser.canvas.items import DesignItem
from snartlaser.canvas.tools import BaseTool, SelectTool, TOOL_REGISTRY
from snartlaser.core.types import ToolType


class DesignScene(QGraphicsScene):
    """Graphics scene for the SnartLaser canvas.

    Attributes:
        workspace_width_mm:  Physical width of the machine work area.
        workspace_height_mm: Physical height of the machine work area.
        show_grid:           Whether to render the background grid.
        grid_spacing_mm:     Distance between grid lines in millimetres.
    """

    #: Emitted when an item is added (carries item_id).
    item_added = pyqtSignal(str)
    #: Emitted when items are removed (carries list of item_ids).
    items_removed = pyqtSignal(list)
    #: Emitted when selection changes (carries list of item_ids).
    selection_changed_signal = pyqtSignal(list)

    def __init__(
        self,
        workspace_width_mm: float = 400.0,
        workspace_height_mm: float = 400.0,
        parent=None,
    ) -> None:
        super().__init__(parent)

        self.workspace_width_mm = workspace_width_mm
        self.workspace_height_mm = workspace_height_mm
        self.show_grid = True
        self.grid_spacing_mm = 10.0

        # Item registry: item_id → DesignItem
        self._items: Dict[str, DesignItem] = {}

        # Active drawing tool
        self._tool: BaseTool = SelectTool(self)

        # Scene rect matches workspace
        self.setSceneRect(QRectF(0, 0, workspace_width_mm, workspace_height_mm))

        self.selectionChanged.connect(self._on_selection_changed)

    # ------------------------------------------------------------------
    # Tool management
    # ------------------------------------------------------------------

    @property
    def active_tool(self) -> BaseTool:
        return self._tool

    def set_tool(self, tool_type: ToolType) -> None:
        """Activate the named tool."""
        self._tool.deactivate()
        cls = TOOL_REGISTRY.get(tool_type, SelectTool)
        self._tool = cls(self)
        self._tool.activate()

    def set_active_color(self, color: str) -> None:
        """Set the colour used by the current drawing tool."""
        self._tool.set_color(color)

    # ------------------------------------------------------------------
    # Item management
    # ------------------------------------------------------------------

    def add_path(
        self, path: QPainterPath, color: str = "#ff0000"
    ) -> DesignItem:
        """Add a :class:`DesignItem` from *path* and return it."""
        item = DesignItem(path, color)
        self.addItem(item)
        self._items[item.item_id] = item
        self.item_added.emit(item.item_id)
        return item

    def add_paths(
        self, paths: List[Tuple[QPainterPath, str]]
    ) -> List[DesignItem]:
        """Add multiple paths at once."""
        added = []
        for path, color in paths:
            added.append(self.add_path(path, color))
        return added

    def remove_item(self, item_id: str) -> None:
        """Remove the item with *item_id* from the scene."""
        item = self._items.pop(item_id, None)
        if item:
            self.removeItem(item)
            self.items_removed.emit([item_id])

    def remove_selected(self) -> None:
        """Delete all selected items."""
        ids = [i.item_id for i in self.selected_design_items()]
        for iid in ids:
            self.remove_item(iid)

    def selected_design_items(self) -> List[DesignItem]:
        return [i for i in self.selectedItems() if isinstance(i, DesignItem)]

    def all_design_items(self) -> List[DesignItem]:
        return list(self._items.values())

    def select_all(self) -> None:
        for item in self._items.values():
            item.setSelected(True)

    def deselect_all(self) -> None:
        self.clearSelection()

    def item_by_id(self, item_id: str) -> Optional[DesignItem]:
        return self._items.get(item_id)

    # ------------------------------------------------------------------
    # Mouse event dispatch
    # ------------------------------------------------------------------

    def mousePressEvent(self, event) -> None:
        pos = event.scenePos()
        # Let select tool handle item picking
        if self._tool.tool_type == ToolType.SELECT:
            super().mousePressEvent(event)
        self._tool.mouse_press(pos, event.button())

    def mouseMoveEvent(self, event) -> None:
        pos = event.scenePos()
        self._tool.mouse_move(pos)
        super().mouseMoveEvent(event)

    def mouseReleaseEvent(self, event) -> None:
        pos = event.scenePos()
        self._tool.mouse_release(pos, event.button())
        if self._tool.tool_type == ToolType.SELECT:
            super().mouseReleaseEvent(event)

    def keyPressEvent(self, event) -> None:
        if event.key() == Qt.Key.Key_Delete:
            self.remove_selected()
        else:
            self._tool.key_press(event.key())
            super().keyPressEvent(event)

    # ------------------------------------------------------------------
    # Drawing (grid & workspace border)
    # ------------------------------------------------------------------

    def drawBackground(self, painter: QPainter, rect: QRectF) -> None:
        # Background fill
        painter.fillRect(rect, QColor("#1a1a2e"))

        scene_rect = self.sceneRect()

        # Workspace fill (slightly lighter)
        painter.fillRect(scene_rect, QColor("#16213e"))

        if self.show_grid:
            self._draw_grid(painter, scene_rect)

        # Workspace border
        border_pen = QPen(QColor("#0f3460"), 0.5)
        border_pen.setCosmetic(True)
        painter.setPen(border_pen)
        painter.drawRect(scene_rect)

    def _draw_grid(self, painter: QPainter, rect: QRectF) -> None:
        sp = self.grid_spacing_mm
        minor_pen = QPen(QColor("#1e2d4a"), 0)
        minor_pen.setCosmetic(True)
        major_pen = QPen(QColor("#2a3f6f"), 0)
        major_pen.setCosmetic(True)

        x = rect.left() - (rect.left() % sp)
        while x <= rect.right():
            is_major = abs(round(x / sp) % 10) == 0
            painter.setPen(major_pen if is_major else minor_pen)
            painter.drawLine(QLineF(x, rect.top(), x, rect.bottom()))
            x += sp

        y = rect.top() - (rect.top() % sp)
        while y <= rect.bottom():
            is_major = abs(round(y / sp) % 10) == 0
            painter.setPen(major_pen if is_major else minor_pen)
            painter.drawLine(QLineF(rect.left(), y, rect.right(), y))
            y += sp

    # ------------------------------------------------------------------
    # Serialisation
    # ------------------------------------------------------------------

    def to_dict(self) -> dict:
        return {
            "workspace_width_mm": self.workspace_width_mm,
            "workspace_height_mm": self.workspace_height_mm,
            "items": [i.to_dict() for i in self._items.values()],
        }

    def load_dict(self, data: dict) -> None:
        """Replace scene content with *data*."""
        self.clear()
        self._items.clear()
        self.workspace_width_mm = data.get("workspace_width_mm", 400.0)
        self.workspace_height_mm = data.get("workspace_height_mm", 400.0)
        self.setSceneRect(
            QRectF(0, 0, self.workspace_width_mm, self.workspace_height_mm)
        )
        for item_data in data.get("items", []):
            item = DesignItem.from_dict(item_data)
            self.addItem(item)
            self._items[item.item_id] = item

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _on_selection_changed(self) -> None:
        ids = [i.item_id for i in self.selected_design_items()]
        self.selection_changed_signal.emit(ids)
