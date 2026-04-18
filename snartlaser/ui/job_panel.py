"""Job panel – workspace and job-level settings widget."""
from __future__ import annotations

from typing import Optional

from PyQt6.QtWidgets import (
    QDoubleSpinBox,
    QFormLayout,
    QGroupBox,
    QLabel,
    QLineEdit,
    QPushButton,
    QTextEdit,
    QVBoxLayout,
    QWidget,
)

from snartlaser.core.event_bus import EventBus
from snartlaser.job.settings import JobSettings


class JobPanel(QWidget):
    """Panel for editing workspace dimensions, material name and notes."""

    def __init__(
        self,
        job: JobSettings,
        event_bus: EventBus,
        parent: Optional[QWidget] = None,
    ) -> None:
        super().__init__(parent)
        self.job = job
        self.event_bus = event_bus
        self._build_ui()

    def _build_ui(self) -> None:
        layout = QVBoxLayout(self)
        layout.setContentsMargins(4, 4, 4, 4)

        title = QLabel("Job Settings")
        title.setStyleSheet("font-weight: bold; font-size: 13px; padding: 4px;")
        layout.addWidget(title)

        # Workspace group
        ws_group = QGroupBox("Workspace")
        ws_form = QFormLayout(ws_group)

        self._width = QDoubleSpinBox()
        self._width.setRange(1.0, 10000.0)
        self._width.setSuffix(" mm")
        self._width.setDecimals(1)
        self._width.setValue(self.job.workspace.width_mm)
        self._width.valueChanged.connect(self._save)
        ws_form.addRow("Width:", self._width)

        self._height = QDoubleSpinBox()
        self._height.setRange(1.0, 10000.0)
        self._height.setSuffix(" mm")
        self._height.setDecimals(1)
        self._height.setValue(self.job.workspace.height_mm)
        self._height.valueChanged.connect(self._save)
        ws_form.addRow("Height:", self._height)

        layout.addWidget(ws_group)

        # Material
        mat_group = QGroupBox("Material")
        mat_form = QFormLayout(mat_group)

        self._material = QLineEdit(self.job.material)
        self._material.setPlaceholderText("e.g. Plywood 3mm")
        self._material.textChanged.connect(self._save)
        mat_form.addRow("Material:", self._material)

        self._notes = QTextEdit(self.job.notes)
        self._notes.setPlaceholderText("Notes…")
        self._notes.setMaximumHeight(80)
        self._notes.textChanged.connect(self._save)
        mat_form.addRow("Notes:", self._notes)

        layout.addWidget(mat_group)
        layout.addStretch()

    def _save(self) -> None:
        self.job.workspace.width_mm = self._width.value()
        self.job.workspace.height_mm = self._height.value()
        self.job.material = self._material.text()
        self.job.notes = self._notes.toPlainText()
        self.event_bus.project_modified.emit(True)

    def refresh(self) -> None:
        self._width.setValue(self.job.workspace.width_mm)
        self._height.setValue(self.job.workspace.height_mm)
        self._material.setText(self.job.material)
        self._notes.setPlainText(self.job.notes)
