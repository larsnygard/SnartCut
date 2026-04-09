"""Main application window.

:class:`MainWindow` is the top-level :class:`QMainWindow` that:

* Hosts the :class:`~snartlaser.ui.canvas_widget.CanvasWidget` as the
  central widget.
* Provides left and right dock panels:

  * **Left** – tool palette (part of the canvas toolbar)
  * **Right (Layers)** – :class:`~snartlaser.ui.layers_panel.LayersPanel`
  * **Right (Job)** – :class:`~snartlaser.ui.job_panel.JobPanel`
  * **Bottom (Device)** – :class:`~snartlaser.ui.device_panel.DevicePanel`

* Implements all menu actions (File, Edit, View, Machine, Help).
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Optional

from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtGui import QAction, QCloseEvent, QIcon, QKeySequence
from PyQt6.QtWidgets import (
    QDockWidget,
    QFileDialog,
    QLabel,
    QMainWindow,
    QMessageBox,
    QStatusBar,
    QTabWidget,
    QWidget,
)

from snartlaser.canvas.scene import DesignScene
from snartlaser.core.config import Config
from snartlaser.core.event_bus import EventBus
from snartlaser.gcode.generator import GCodeGenerator
from snartlaser.job.settings import JobSettings
from snartlaser.ui.canvas_widget import CanvasWidget
from snartlaser.ui.device_panel import DevicePanel
from snartlaser.ui.job_panel import JobPanel
from snartlaser.ui.layers_panel import LayersPanel


class MainWindow(QMainWindow):
    """SnartLaser main window."""

    def __init__(
        self,
        config: Config,
        event_bus: EventBus,
        parent: Optional[QWidget] = None,
    ) -> None:
        super().__init__(parent)
        self.config = config
        self.event_bus = event_bus

        self.job = JobSettings()
        self.job.workspace.width_mm = config.workspace_width_mm
        self.job.workspace.height_mm = config.workspace_height_mm

        self.scene = DesignScene(
            self.job.workspace.width_mm,
            self.job.workspace.height_mm,
        )

        self._modified = False
        self._current_file: Optional[str] = None

        self._build_ui()
        self._build_menu()
        self._connect_signals()
        self._apply_theme()

        self.setWindowTitle("SnartLaser")
        self.resize(1280, 800)

    # ------------------------------------------------------------------
    # UI construction
    # ------------------------------------------------------------------

    def _build_ui(self) -> None:
        # Central widget – canvas
        self._canvas = CanvasWidget(self.scene, self.event_bus)
        self.setCentralWidget(self._canvas)

        # Right dock tabs (Layers + Job)
        right_tabs = QTabWidget()
        right_tabs.setTabPosition(QTabWidget.TabPosition.North)

        self._layers_panel = LayersPanel(self.job, self.event_bus)
        right_tabs.addTab(self._layers_panel, "Layers")

        self._job_panel = JobPanel(self.job, self.event_bus)
        right_tabs.addTab(self._job_panel, "Job")

        right_dock = QDockWidget("Design", self)
        right_dock.setObjectName("right_dock")
        right_dock.setWidget(right_tabs)
        right_dock.setFeatures(
            QDockWidget.DockWidgetFeature.DockWidgetMovable
            | QDockWidget.DockWidgetFeature.DockWidgetFloatable
        )
        self.addDockWidget(Qt.DockWidgetArea.RightDockWidgetArea, right_dock)

        # Bottom dock – device panel
        self._device_panel = DevicePanel(self.config, self.event_bus)
        device_dock = QDockWidget("Device", self)
        device_dock.setObjectName("device_dock")
        device_dock.setWidget(self._device_panel)
        device_dock.setFeatures(
            QDockWidget.DockWidgetFeature.DockWidgetMovable
            | QDockWidget.DockWidgetFeature.DockWidgetFloatable
        )
        self.addDockWidget(Qt.DockWidgetArea.BottomDockWidgetArea, device_dock)

        # Status bar
        self._status_bar = QStatusBar()
        self._coord_label = QLabel("X: 0.000  Y: 0.000")
        self._zoom_label = QLabel("Zoom: 100%")
        self._status_bar.addPermanentWidget(self._coord_label)
        self._status_bar.addPermanentWidget(self._zoom_label)
        self.setStatusBar(self._status_bar)

    def _build_menu(self) -> None:
        menu_bar = self.menuBar()

        # ---- File ----
        file_menu = menu_bar.addMenu("&File")

        new_act = QAction("&New", self)
        new_act.setShortcut(QKeySequence.StandardKey.New)
        new_act.triggered.connect(self._new_file)
        file_menu.addAction(new_act)

        open_act = QAction("&Open…", self)
        open_act.setShortcut(QKeySequence.StandardKey.Open)
        open_act.triggered.connect(self._open_file)
        file_menu.addAction(open_act)

        file_menu.addSeparator()

        import_svg = QAction("Import SVG…", self)
        import_svg.triggered.connect(lambda: self._import_file("svg"))
        file_menu.addAction(import_svg)

        import_dxf = QAction("Import DXF…", self)
        import_dxf.triggered.connect(lambda: self._import_file("dxf"))
        file_menu.addAction(import_dxf)

        file_menu.addSeparator()

        save_act = QAction("&Save", self)
        save_act.setShortcut(QKeySequence.StandardKey.Save)
        save_act.triggered.connect(self._save_file)
        file_menu.addAction(save_act)

        save_as_act = QAction("Save &As…", self)
        save_as_act.setShortcut(QKeySequence.StandardKey.SaveAs)
        save_as_act.triggered.connect(self._save_file_as)
        file_menu.addAction(save_as_act)

        file_menu.addSeparator()

        export_svg = QAction("Export SVG…", self)
        export_svg.triggered.connect(self._export_svg)
        file_menu.addAction(export_svg)

        export_gcode = QAction("Export G-code…", self)
        export_gcode.triggered.connect(self._export_gcode)
        file_menu.addAction(export_gcode)

        file_menu.addSeparator()

        quit_act = QAction("&Quit", self)
        quit_act.setShortcut(QKeySequence.StandardKey.Quit)
        quit_act.triggered.connect(self.close)
        file_menu.addAction(quit_act)

        # ---- Edit ----
        edit_menu = menu_bar.addMenu("&Edit")

        sel_all = QAction("Select &All", self)
        sel_all.setShortcut(QKeySequence.StandardKey.SelectAll)
        sel_all.triggered.connect(self.scene.select_all)
        edit_menu.addAction(sel_all)

        delete_act = QAction("&Delete", self)
        delete_act.setShortcut(QKeySequence.StandardKey.Delete)
        delete_act.triggered.connect(self.scene.remove_selected)
        edit_menu.addAction(delete_act)

        # ---- View ----
        view_menu = menu_bar.addMenu("&View")

        zoom_in_act = QAction("Zoom &In", self)
        zoom_in_act.setShortcut(QKeySequence.StandardKey.ZoomIn)
        zoom_in_act.triggered.connect(self._canvas.view.zoom_in)
        view_menu.addAction(zoom_in_act)

        zoom_out_act = QAction("Zoom &Out", self)
        zoom_out_act.setShortcut(QKeySequence.StandardKey.ZoomOut)
        zoom_out_act.triggered.connect(self._canvas.view.zoom_out)
        view_menu.addAction(zoom_out_act)

        zoom_fit_act = QAction("&Fit to Window", self)
        zoom_fit_act.setShortcut(QKeySequence("Ctrl+0"))
        zoom_fit_act.triggered.connect(self._canvas.view.zoom_fit)
        view_menu.addAction(zoom_fit_act)

        view_menu.addSeparator()

        grid_act = QAction("Show &Grid", self)
        grid_act.setCheckable(True)
        grid_act.setChecked(self.config.show_grid)
        grid_act.triggered.connect(self._toggle_grid)
        view_menu.addAction(grid_act)

        # ---- Machine ----
        machine_menu = menu_bar.addMenu("&Machine")

        conn_act = QAction("&Connect…", self)
        conn_act.triggered.connect(self._device_panel._open_settings)
        machine_menu.addAction(conn_act)

        run_act = QAction("&Run Job", self)
        run_act.setShortcut(QKeySequence("F5"))
        run_act.triggered.connect(self._run_job)
        machine_menu.addAction(run_act)

        home_act = QAction("&Home", self)
        home_act.triggered.connect(self._device_panel._home)
        machine_menu.addAction(home_act)

        # ---- Help ----
        help_menu = menu_bar.addMenu("&Help")
        about_act = QAction("&About SnartLaser", self)
        about_act.triggered.connect(self._show_about)
        help_menu.addAction(about_act)

    def _connect_signals(self) -> None:
        self.event_bus.project_modified.connect(self._on_modified)
        self.event_bus.zoom_changed.connect(
            lambda f: self._zoom_label.setText(f"Zoom: {int(f * 100)}%")
        )
        self.event_bus.position_changed.connect(
            lambda x, y: self._coord_label.setText(f"X: {x:.3f}  Y: {y:.3f}")
        )
        self.scene.item_added.connect(
            lambda _: self.event_bus.project_modified.emit(True)
        )

    # ------------------------------------------------------------------
    # Theme
    # ------------------------------------------------------------------

    def _apply_theme(self) -> None:
        self.setStyleSheet("""
            QMainWindow, QWidget {
                background-color: #1a1a2e;
                color: #e0e0e0;
            }
            QMenuBar {
                background-color: #16213e;
                color: #e0e0e0;
            }
            QMenuBar::item:selected {
                background-color: #0f3460;
            }
            QMenu {
                background-color: #16213e;
                color: #e0e0e0;
                border: 1px solid #0f3460;
            }
            QMenu::item:selected {
                background-color: #0f3460;
            }
            QDockWidget {
                color: #e0e0e0;
                font-weight: bold;
            }
            QDockWidget::title {
                background-color: #16213e;
                padding: 4px;
            }
            QToolBar {
                background-color: #16213e;
                border-bottom: 1px solid #0f3460;
                spacing: 3px;
            }
            QToolButton {
                background-color: transparent;
                color: #e0e0e0;
                border: 1px solid transparent;
                border-radius: 3px;
                padding: 3px 6px;
                font-size: 14px;
            }
            QToolButton:hover {
                background-color: #0f3460;
                border-color: #4a90d9;
            }
            QToolButton:checked {
                background-color: #0f3460;
                border-color: #4a90d9;
            }
            QPushButton {
                background-color: #16213e;
                color: #e0e0e0;
                border: 1px solid #0f3460;
                border-radius: 4px;
                padding: 4px 10px;
            }
            QPushButton:hover {
                background-color: #0f3460;
            }
            QPushButton:disabled {
                color: #555;
                border-color: #333;
            }
            QGroupBox {
                border: 1px solid #0f3460;
                border-radius: 4px;
                margin-top: 8px;
                padding-top: 8px;
                color: #aaa;
            }
            QGroupBox::title {
                subcontrol-origin: margin;
                left: 8px;
                padding: 0 4px;
            }
            QListWidget, QTextEdit, QLineEdit, QSpinBox, QDoubleSpinBox, QComboBox {
                background-color: #0f1b2e;
                color: #e0e0e0;
                border: 1px solid #0f3460;
                border-radius: 3px;
            }
            QScrollBar:vertical, QScrollBar:horizontal {
                background: #16213e;
                width: 10px;
            }
            QScrollBar::handle {
                background: #0f3460;
                border-radius: 4px;
            }
            QTabWidget::pane {
                border: 1px solid #0f3460;
            }
            QTabBar::tab {
                background-color: #16213e;
                color: #aaa;
                padding: 4px 12px;
                border: 1px solid #0f3460;
            }
            QTabBar::tab:selected {
                background-color: #0f3460;
                color: #e0e0e0;
            }
            QStatusBar {
                background-color: #16213e;
                color: #aaa;
            }
            QProgressBar {
                background-color: #0f1b2e;
                border: 1px solid #0f3460;
                border-radius: 3px;
                text-align: center;
                color: #e0e0e0;
            }
            QProgressBar::chunk {
                background-color: #2ecc71;
            }
        """)

    # ------------------------------------------------------------------
    # File actions
    # ------------------------------------------------------------------

    def _new_file(self) -> None:
        if not self._confirm_discard():
            return
        self.scene.clear()
        self.scene._items.clear()
        self.job = JobSettings()
        self.job.workspace.width_mm = self.config.workspace_width_mm
        self.job.workspace.height_mm = self.config.workspace_height_mm
        self._layers_panel.job = self.job
        self._layers_panel.refresh()
        self._job_panel.job = self.job
        self._job_panel.refresh()
        self._current_file = None
        self._set_modified(False)

    def _open_file(self) -> None:
        if not self._confirm_discard():
            return
        path, _ = QFileDialog.getOpenFileName(
            self,
            "Open Project",
            self.config.last_directory,
            "SnartLaser Project (*.slp);;All Files (*)",
        )
        if not path:
            return
        try:
            with open(path) as f:
                data = json.load(f)
            scene_data = data.get("scene", {})
            job_data = data.get("job", {})
            self.scene.load_dict(scene_data)
            self.job = JobSettings.from_dict(job_data)
            self._layers_panel.job = self.job
            self._layers_panel.refresh()
            self._job_panel.job = self.job
            self._job_panel.refresh()
            self._current_file = path
            self.config.add_recent_file(path)
            self.config.last_directory = str(Path(path).parent)
            self._set_modified(False)
            self.event_bus.file_opened.emit(path)
        except Exception as exc:
            QMessageBox.critical(self, "Open Error", str(exc))

    def _save_file(self) -> None:
        if self._current_file:
            self._do_save(self._current_file)
        else:
            self._save_file_as()

    def _save_file_as(self) -> None:
        path, _ = QFileDialog.getSaveFileName(
            self,
            "Save Project",
            self.config.last_directory,
            "SnartLaser Project (*.slp);;All Files (*)",
        )
        if path:
            if not path.endswith(".slp"):
                path += ".slp"
            self._do_save(path)

    def _do_save(self, path: str) -> None:
        try:
            data = {
                "scene": self.scene.to_dict(),
                "job": self.job.to_dict(),
            }
            with open(path, "w") as f:
                json.dump(data, f, indent=2)
            self._current_file = path
            self.config.add_recent_file(path)
            self.config.last_directory = str(Path(path).parent)
            self._set_modified(False)
            self.event_bus.file_saved.emit(path)
        except Exception as exc:
            QMessageBox.critical(self, "Save Error", str(exc))

    def _import_file(self, fmt: str) -> None:
        filters = {
            "svg": "SVG Files (*.svg);;All Files (*)",
            "dxf": "DXF Files (*.dxf);;All Files (*)",
        }
        path, _ = QFileDialog.getOpenFileName(
            self, f"Import {fmt.upper()}", self.config.last_directory, filters[fmt]
        )
        if not path:
            return
        try:
            if fmt == "svg":
                from snartlaser.formats.svg import load as svg_load
                paths = svg_load(path)
            else:
                from snartlaser.formats.dxf import load as dxf_load
                paths = dxf_load(path)
            self.scene.add_paths(paths)
            self.config.last_directory = str(Path(path).parent)
            self._set_modified(True)
            self._status_bar.showMessage(
                f"Imported {len(paths)} paths from {Path(path).name}", 3000
            )
        except Exception as exc:
            QMessageBox.critical(self, "Import Error", str(exc))

    def _export_svg(self) -> None:
        path, _ = QFileDialog.getSaveFileName(
            self, "Export SVG", self.config.last_directory, "SVG Files (*.svg)"
        )
        if not path:
            return
        try:
            from snartlaser.formats.svg import save as svg_save
            pairs = [(i.path(), i.layer_color) for i in self.scene.all_design_items()]
            svg_save(
                pairs,
                self.job.workspace.width_mm,
                self.job.workspace.height_mm,
                path,
            )
            self._status_bar.showMessage(f"Exported to {path}", 3000)
        except Exception as exc:
            QMessageBox.critical(self, "Export Error", str(exc))

    def _export_gcode(self) -> None:
        path, _ = QFileDialog.getSaveFileName(
            self,
            "Export G-code",
            self.config.last_directory,
            "G-code (*.gcode *.nc *.txt);;All Files (*)",
        )
        if not path:
            return
        try:
            gcode = self._build_gcode()
            Path(path).write_text(gcode)
            self._status_bar.showMessage(f"G-code exported to {path}", 3000)
        except Exception as exc:
            QMessageBox.critical(self, "Export Error", str(exc))

    # ------------------------------------------------------------------
    # Machine actions
    # ------------------------------------------------------------------

    def _run_job(self) -> None:
        """Generate G-code from the current scene + job and send it to the device."""
        device = self._device_panel._device
        if device is None or not device.is_connected:
            QMessageBox.warning(
                self, "Not Connected", "Please connect to a device first."
            )
            return
        gcode_str = self._build_gcode()
        lines = [l for l in gcode_str.split("\n") if l.strip() and not l.startswith(";")]
        total = len(lines)
        device.send_job(lines)
        self._status_bar.showMessage(f"Sending {total} lines to device…")

    def _build_gcode(self) -> str:
        from snartlaser.core.types import DeviceType as DT
        dtype_str = self.config.device_type
        try:
            dtype = DT(dtype_str)
        except ValueError:
            dtype = DT.GRBL_LASER

        gen = GCodeGenerator(
            device_type=dtype,
            workspace_width_mm=self.job.workspace.width_mm,
            workspace_height_mm=self.job.workspace.height_mm,
        )

        layer_data = []
        for layer in self.job.layers.enabled_layers:
            paths = [
                self.scene.item_by_id(iid).path()
                for iid in layer.item_ids
                if self.scene.item_by_id(iid)
            ]
            if not paths:
                # Use all scene items for layers without explicit assignment
                paths = [i.path() for i in self.scene.all_design_items()]
            layer_data.append((paths, layer.settings))

        if not layer_data:
            # No layers? create a default pass using all items
            from snartlaser.core.types import CutSettings
            paths = [i.path() for i in self.scene.all_design_items()]
            layer_data = [(paths, CutSettings())]

        return gen.generate_string(layer_data)

    # ------------------------------------------------------------------
    # View actions
    # ------------------------------------------------------------------

    def _toggle_grid(self, show: bool) -> None:
        self.scene.show_grid = show
        self.config.show_grid = show
        self.scene.update()

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------

    def _on_modified(self, modified: bool) -> None:
        self._set_modified(modified)

    def _set_modified(self, modified: bool) -> None:
        self._modified = modified
        title = "SnartLaser"
        if self._current_file:
            title += f" – {Path(self._current_file).name}"
        if modified:
            title += " *"
        self.setWindowTitle(title)

    def _confirm_discard(self) -> bool:
        if not self._modified:
            return True
        reply = QMessageBox.question(
            self,
            "Unsaved Changes",
            "There are unsaved changes. Discard and continue?",
            QMessageBox.StandardButton.Discard | QMessageBox.StandardButton.Cancel,
        )
        return reply == QMessageBox.StandardButton.Discard

    def _show_about(self) -> None:
        QMessageBox.about(
            self,
            "About SnartLaser",
            "<b>SnartLaser v0.1.0</b><br><br>"
            "An open-source laser cutter and vinyl cutter design application.<br>"
            "Runs natively on Linux, Windows and macOS.<br><br>"
            "Licensed under the MIT License.",
        )

    def closeEvent(self, event: QCloseEvent) -> None:
        if self._confirm_discard():
            event.accept()
        else:
            event.ignore()
