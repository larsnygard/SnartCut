"""Layers panel – sidebar showing the cut-layer list.

Users can:

* Add / delete layers
* Reorder layers via drag-and-drop (future)
* Edit layer name, mode, speed and power inline
* Apply material presets via button
"""
from __future__ import annotations

from typing import Optional

from PyQt6.QtCore import Qt, pyqtSignal
from PyQt6.QtGui import QColor
from PyQt6.QtWidgets import (
    QAbstractItemView,
    QComboBox,
    QDoubleSpinBox,
    QFormLayout,
    QFrame,
    QGroupBox,
    QHBoxLayout,
    QLabel,
    QListWidget,
    QListWidgetItem,
    QPushButton,
    QSpinBox,
    QVBoxLayout,
    QWidget,
)

from snartlaser.core.event_bus import EventBus
from snartlaser.core.types import CutSettings, LayerMode
from snartlaser.job.layer import LayerList
from snartlaser.job.settings import JobSettings


class _LayerItem(QListWidgetItem):
    def __init__(self, index: int, layer_name: str, color: str) -> None:
        super().__init__(f"  {layer_name}")
        self.layer_index = index
        self.setForeground(QColor(color))


class LayersPanel(QWidget):
    """Sidebar panel for managing cut layers."""

    def __init__(
        self,
        job: JobSettings,
        event_bus: EventBus,
        parent: Optional[QWidget] = None,
    ) -> None:
        super().__init__(parent)
        self.job = job
        self.event_bus = event_bus
        self._building = False
        self._build_ui()
        self._refresh_list()

    # ------------------------------------------------------------------
    # UI construction
    # ------------------------------------------------------------------

    def _build_ui(self) -> None:
        layout = QVBoxLayout(self)
        layout.setContentsMargins(4, 4, 4, 4)

        title = QLabel("Cut Layers")
        title.setStyleSheet("font-weight: bold; font-size: 13px; padding: 4px;")
        layout.addWidget(title)

        # Layer list
        self._list = QListWidget()
        self._list.setSelectionMode(
            QAbstractItemView.SelectionMode.SingleSelection
        )
        self._list.currentRowChanged.connect(self._on_row_changed)
        layout.addWidget(self._list)

        # Buttons
        btn_row = QHBoxLayout()
        self._add_btn = QPushButton("+")
        self._add_btn.setToolTip("Add layer")
        self._add_btn.setFixedWidth(32)
        self._add_btn.clicked.connect(self._add_layer)
        btn_row.addWidget(self._add_btn)

        self._del_btn = QPushButton("−")
        self._del_btn.setToolTip("Remove layer")
        self._del_btn.setFixedWidth(32)
        self._del_btn.clicked.connect(self._del_layer)
        btn_row.addWidget(self._del_btn)

        self._preset_btn = QPushButton("Preset…")
        self._preset_btn.setToolTip("Apply material preset")
        self._preset_btn.clicked.connect(self._open_preset_dialog)
        btn_row.addWidget(self._preset_btn)

        btn_row.addStretch()
        layout.addLayout(btn_row)

        # Layer settings editor
        self._editor = QGroupBox("Layer Settings")
        self._editor.setEnabled(False)
        form = QFormLayout(self._editor)

        self._mode_cb = QComboBox()
        for m in LayerMode:
            self._mode_cb.addItem(m.value.replace("_", " ").title(), m.value)
        self._mode_cb.currentIndexChanged.connect(self._save_current)
        form.addRow("Mode:", self._mode_cb)

        self._speed_spin = QDoubleSpinBox()
        self._speed_spin.setRange(0.1, 10000.0)
        self._speed_spin.setSuffix(" mm/s")
        self._speed_spin.setDecimals(1)
        self._speed_spin.valueChanged.connect(self._save_current)
        form.addRow("Speed:", self._speed_spin)

        self._power_spin = QDoubleSpinBox()
        self._power_spin.setRange(0.0, 100.0)
        self._power_spin.setSuffix(" %")
        self._power_spin.setDecimals(1)
        self._power_spin.valueChanged.connect(self._save_current)
        form.addRow("Power:", self._power_spin)

        self._passes_spin = QSpinBox()
        self._passes_spin.setRange(1, 99)
        self._passes_spin.valueChanged.connect(self._save_current)
        form.addRow("Passes:", self._passes_spin)

        layout.addWidget(self._editor)

    # ------------------------------------------------------------------
    # Refresh
    # ------------------------------------------------------------------

    def _refresh_list(self) -> None:
        self._list.clear()
        for i, layer in enumerate(self.job.layers):
            self._list.addItem(_LayerItem(i, layer.name, layer.color))
        if self._list.count() > 0:
            self._list.setCurrentRow(0)

    def _load_layer_into_editor(self, index: int) -> None:
        self._building = True
        s = self.job.layers[index].settings
        idx = self._mode_cb.findData(s.mode.value)
        if idx >= 0:
            self._mode_cb.setCurrentIndex(idx)
        self._speed_spin.setValue(s.speed_mm_s)
        self._power_spin.setValue(s.power_pct)
        self._passes_spin.setValue(s.passes)
        self._building = False

    # ------------------------------------------------------------------
    # Slots
    # ------------------------------------------------------------------

    def _on_row_changed(self, row: int) -> None:
        has_row = 0 <= row < len(self.job.layers)
        self._editor.setEnabled(has_row)
        if has_row:
            self._load_layer_into_editor(row)

    def _save_current(self) -> None:
        if self._building:
            return
        row = self._list.currentRow()
        if row < 0 or row >= len(self.job.layers):
            return
        s = self.job.layers[row].settings
        s.mode = LayerMode(self._mode_cb.currentData())
        s.speed_mm_s = self._speed_spin.value()
        s.power_pct = self._power_spin.value()
        s.passes = self._passes_spin.value()
        self.event_bus.layer_updated.emit(row)

    def _add_layer(self) -> None:
        self.job.layers.add()
        self._refresh_list()
        self._list.setCurrentRow(len(self.job.layers) - 1)
        self.event_bus.layers_changed.emit()

    def _del_layer(self) -> None:
        row = self._list.currentRow()
        if row < 0 or len(self.job.layers) == 0:
            return
        self.job.layers.remove(row)
        self._refresh_list()
        self.event_bus.layers_changed.emit()

    def _open_preset_dialog(self) -> None:
        from snartlaser.ui.dialogs.material_library import MaterialLibraryDialog

        dlg = MaterialLibraryDialog(self)
        if dlg.exec() and dlg.selected_preset:
            self.job.apply_preset(dlg.selected_preset)
            self._refresh_list()
            self._list.setCurrentRow(len(self.job.layers) - 1)
            self.event_bus.layers_changed.emit()

    # ------------------------------------------------------------------
    # Public
    # ------------------------------------------------------------------

    def refresh(self) -> None:
        """Rebuild the list from the current job state."""
        self._refresh_list()
