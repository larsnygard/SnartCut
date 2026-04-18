"""Custom :class:`~PyQt6.QtWidgets.QGraphicsItem` subclasses.

Each item wraps a :class:`~PyQt6.QtGui.QPainterPath` and carries an
application-level ``item_id`` (UUID string).  Items store their own
display colour which mirrors the layer colour.
"""
from __future__ import annotations

import uuid
from typing import Optional

from PyQt6.QtCore import QRectF, Qt
from PyQt6.QtGui import (
    QBrush,
    QColor,
    QPainter,
    QPainterPath,
    QPen,
    QTransform,
)
from PyQt6.QtWidgets import (
    QGraphicsItem,
    QGraphicsPathItem,
    QStyleOptionGraphicsItem,
    QWidget,
)

# Selection handle size in scene units (mm)
_HANDLE_SIZE = 3.0


class DesignItem(QGraphicsPathItem):
    """A path-based design item with a unique ID and layer colour.

    Attributes:
        item_id: Unique string identifier (UUID4).
        layer_color: Hex colour string used for the stroke pen.
    """

    def __init__(
        self,
        path: QPainterPath,
        color: str = "#ff0000",
        item_id: Optional[str] = None,
    ) -> None:
        super().__init__(path)
        self.item_id: str = item_id or str(uuid.uuid4())
        self.layer_color: str = color

        self._update_pen()

        self.setFlag(QGraphicsItem.GraphicsItemFlag.ItemIsSelectable, True)
        self.setFlag(QGraphicsItem.GraphicsItemFlag.ItemIsMovable, True)
        self.setFlag(
            QGraphicsItem.GraphicsItemFlag.ItemSendsGeometryChanges, True
        )
        self.setAcceptHoverEvents(True)

    # ------------------------------------------------------------------
    # Appearance
    # ------------------------------------------------------------------

    def set_color(self, color: str) -> None:
        """Update the stroke colour and redraw."""
        self.layer_color = color
        self._update_pen()

    def _update_pen(self) -> None:
        pen = QPen(QColor(self.layer_color))
        pen.setWidthF(0.3)  # ~0.3 mm stroke
        pen.setCosmetic(False)
        self.setPen(pen)
        self.setBrush(QBrush(Qt.BrushStyle.NoBrush))

    # ------------------------------------------------------------------
    # Painting
    # ------------------------------------------------------------------

    def paint(
        self,
        painter: QPainter,
        option: QStyleOptionGraphicsItem,
        widget: Optional[QWidget] = None,
    ) -> None:
        super().paint(painter, option, widget)

        # Draw selection handles at bounding-box corners
        if self.isSelected():
            painter.save()
            painter.setPen(QPen(QColor("#0078d4"), 0))
            painter.setBrush(QBrush(QColor("#0078d4")))
            r = self.boundingRect()
            hs = _HANDLE_SIZE
            for hx, hy in [
                (r.left(), r.top()),
                (r.right(), r.top()),
                (r.left(), r.bottom()),
                (r.right(), r.bottom()),
            ]:
                painter.drawRect(
                    QRectF(hx - hs / 2, hy - hs / 2, hs, hs)
                )
            painter.restore()

    # ------------------------------------------------------------------
    # Serialisation
    # ------------------------------------------------------------------

    def to_dict(self) -> dict:
        """Serialise this item to a JSON-compatible dictionary."""
        # Store path as list of (type, x, y) elements
        elements = []
        path = self.path()
        from PyQt6.QtGui import QPainterPath as _QP
        for i in range(path.elementCount()):
            e = path.elementAt(i)
            elements.append((e.type.value, e.x, e.y))
        t = self.transform()
        return {
            "item_id": self.item_id,
            "color": self.layer_color,
            "path_elements": elements,
            "pos": (self.x(), self.y()),
        }

    @classmethod
    def from_dict(cls, data: dict) -> "DesignItem":
        from PyQt6.QtGui import QPainterPath as _QP
        path = _QP()
        for etype, x, y in data.get("path_elements", []):
            t = _QP.ElementType(etype)
            if t == _QP.ElementType.MoveToElement:
                path.moveTo(x, y)
            elif t == _QP.ElementType.LineToElement:
                path.lineTo(x, y)
            elif t in (_QP.ElementType.CurveToElement, _QP.ElementType.CurveToDataElement):
                path.cubicTo(x, y, x, y, x, y)  # simplified
        item = cls(path, data.get("color", "#ff0000"), data.get("item_id"))
        px, py = data.get("pos", (0.0, 0.0))
        item.setPos(px, py)
        return item
