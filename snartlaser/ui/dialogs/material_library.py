"""Material library dialog.

Displays the built-in material presets and lets the user apply one to
the current job.
"""
from __future__ import annotations

from typing import Optional

from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import (
    QDialog,
    QDialogButtonBox,
    QHBoxLayout,
    QLabel,
    QListWidget,
    QListWidgetItem,
    QTextEdit,
    QVBoxLayout,
    QWidget,
)

from snartlaser.job.settings import MATERIAL_LIBRARY


class MaterialLibraryDialog(QDialog):
    """Modal dialog for selecting a material preset."""

    def __init__(self, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self.setWindowTitle("Material Library")
        self.setMinimumSize(520, 360)
        self.selected_preset: Optional[str] = None
        self._build_ui()

    def _build_ui(self) -> None:
        layout = QVBoxLayout(self)

        top = QHBoxLayout()

        # List of presets
        self._list = QListWidget()
        for name in MATERIAL_LIBRARY:
            self._list.addItem(QListWidgetItem(name))
        self._list.currentRowChanged.connect(self._on_selection)
        top.addWidget(self._list, 1)

        # Details pane
        self._details = QTextEdit()
        self._details.setReadOnly(True)
        top.addWidget(self._details, 2)
        layout.addLayout(top)

        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        buttons.accepted.connect(self._apply)
        buttons.rejected.connect(self.reject)
        layout.addWidget(buttons)

        if self._list.count() > 0:
            self._list.setCurrentRow(0)

    def _on_selection(self, row: int) -> None:
        if row < 0:
            self._details.clear()
            return
        name = self._list.item(row).text()
        cs = MATERIAL_LIBRARY[name]
        html = f"""<b>{name}</b><br>
<table>
<tr><td>Mode:</td><td>{cs.mode.value}</td></tr>
<tr><td>Speed:</td><td>{cs.speed_mm_s} mm/s</td></tr>
<tr><td>Power:</td><td>{cs.power_pct}%</td></tr>
<tr><td>Passes:</td><td>{cs.passes}</td></tr>
<tr><td>Air assist:</td><td>{cs.air_assist.value}</td></tr>
</table>"""
        self._details.setHtml(html)

    def _apply(self) -> None:
        row = self._list.currentRow()
        if row >= 0:
            self.selected_preset = self._list.item(row).text()
        self.accept()
