"""Canvas widget – integrates :class:`~snartlaser.canvas.scene.DesignScene`
and :class:`~snartlaser.canvas.view.DesignView` with an inline toolbar.
"""
from __future__ import annotations

from typing import Optional

from PyQt6.QtCore import Qt, pyqtSignal
from PyQt6.QtGui import QColor, QIcon, QKeySequence
from PyQt6.QtWidgets import (
    QColorDialog,
    QHBoxLayout,
    QLabel,
    QPushButton,
    QSizePolicy,
    QToolBar,
    QToolButton,
    QVBoxLayout,
    QWidget,
)

from snartlaser.canvas.scene import DesignScene
from snartlaser.canvas.view import DesignView
from snartlaser.core.event_bus import EventBus
from snartlaser.core.types import ToolType


# Tool button metadata: (ToolType, label, tooltip, shortcut)
_TOOLS = [
    (ToolType.SELECT,    "↖",  "Select / Move (S)",    "S"),
    (ToolType.PAN,       "✋",  "Pan (H)",              "H"),
    (ToolType.RECTANGLE, "▭",  "Rectangle (R)",        "R"),
    (ToolType.ELLIPSE,   "◯",  "Ellipse (E)",          "E"),
    (ToolType.LINE,      "╱",  "Line (L)",             "L"),
]


class CanvasWidget(QWidget):
    """Widget combining the toolbar, status bar and :class:`DesignView`."""

    def __init__(
        self,
        scene: DesignScene,
        event_bus: EventBus,
        parent: Optional[QWidget] = None,
    ) -> None:
        super().__init__(parent)
        self.scene = scene
        self.event_bus = event_bus
        self._active_color = "#ff0000"
        self._build_ui()
        self._connect_signals()

    # ------------------------------------------------------------------
    # UI construction
    # ------------------------------------------------------------------

    def _build_ui(self) -> None:
        root = QVBoxLayout(self)
        root.setSpacing(0)
        root.setContentsMargins(0, 0, 0, 0)

        # Tool bar
        toolbar = QToolBar("Tools")
        toolbar.setMovable(False)
        toolbar.setOrientation(Qt.Orientation.Horizontal)

        self._tool_buttons: dict[ToolType, QToolButton] = {}
        for tool_type, label, tip, shortcut in _TOOLS:
            btn = QToolButton()
            btn.setText(label)
            btn.setToolTip(tip)
            btn.setCheckable(True)
            btn.setShortcut(QKeySequence(shortcut))
            btn.clicked.connect(lambda checked, t=tool_type: self._activate_tool(t))
            toolbar.addWidget(btn)
            self._tool_buttons[tool_type] = btn

        toolbar.addSeparator()

        # Colour picker button
        self._color_btn = QPushButton()
        self._color_btn.setFixedSize(24, 24)
        self._color_btn.setToolTip("Stroke colour")
        self._set_color_btn_style(self._active_color)
        self._color_btn.clicked.connect(self._pick_color)
        toolbar.addWidget(self._color_btn)

        toolbar.addSeparator()

        # Zoom controls
        zoom_in = QToolButton()
        zoom_in.setText("+")
        zoom_in.setToolTip("Zoom in (Ctrl+=)")
        zoom_in.clicked.connect(lambda: self.view.zoom_in())
        toolbar.addWidget(zoom_in)

        zoom_out = QToolButton()
        zoom_out.setText("−")
        zoom_out.setToolTip("Zoom out (Ctrl+-)")
        zoom_out.clicked.connect(lambda: self.view.zoom_out())
        toolbar.addWidget(zoom_out)

        zoom_fit = QToolButton()
        zoom_fit.setText("⊡")
        zoom_fit.setToolTip("Fit to window (Ctrl+0)")
        zoom_fit.clicked.connect(lambda: self.view.zoom_fit())
        toolbar.addWidget(zoom_fit)

        root.addWidget(toolbar)

        # Canvas view
        self.view = DesignView(self.scene)
        self.view.zoom_changed.connect(self._on_zoom)
        root.addWidget(self.view)

        # Status bar
        self._status = QLabel("Ready")
        self._status.setStyleSheet("color: #aaa; padding: 2px 6px;")
        root.addWidget(self._status)

        # Activate SELECT tool by default
        self._activate_tool(ToolType.SELECT)

    def _connect_signals(self) -> None:
        self.event_bus.tool_changed.connect(
            lambda name: self._on_external_tool_change(name)
        )

    # ------------------------------------------------------------------
    # Slots
    # ------------------------------------------------------------------

    def _activate_tool(self, tool_type: ToolType) -> None:
        self.scene.set_tool(tool_type)
        for t, btn in self._tool_buttons.items():
            btn.setChecked(t == tool_type)
        self.event_bus.tool_changed.emit(tool_type.name)

    def _pick_color(self) -> None:
        color = QColorDialog.getColor(
            QColor(self._active_color), self, "Stroke Colour"
        )
        if color.isValid():
            self._active_color = color.name()
            self._set_color_btn_style(self._active_color)
            self.scene.set_active_color(self._active_color)

    def _set_color_btn_style(self, color: str) -> None:
        self._color_btn.setStyleSheet(
            f"background-color: {color}; border: 1px solid #555; border-radius: 3px;"
        )

    def _on_zoom(self, factor: float) -> None:
        pct = int(factor * 100)
        self._status.setText(f"Zoom: {pct}%")
        self.event_bus.zoom_changed.emit(factor)

    def _on_external_tool_change(self, name: str) -> None:
        try:
            tt = ToolType[name]
            self._activate_tool(tt)
        except KeyError:
            pass
