# SnartLaser

**Open-source laser cutter and vinyl cutter design & control application.**

SnartLaser is a cross-platform desktop application inspired by LightBurn.
It runs **natively on Linux** (and also on Windows and macOS) and is licensed
under the permissive MIT license.

---

## Features

| Category | Details |
|---|---|
| **Design canvas** | 2-D vector editor – rectangle, ellipse, line tools; select & move; zoom/pan |
| **File formats** | Import/export SVG and DXF; save/load native `.slp` project files |
| **Cut layers** | Ordered layer list with per-layer speed, power, passes, mode (line/fill) |
| **Material library** | Built-in presets for plywood, acrylic, cardboard, leather, vinyl |
| **G-code output** | GRBL-compatible G-code for laser cutters; raster fill for engraving |
| **HPGL output** | Pen-up/pen-down HPGL for vinyl cutters |
| **Device control** | Serial connection to GRBL laser cutters and HPGL vinyl cutters |
| **Real-time status** | Machine position, jog controls, job progress, console log |
| **Dark UI** | Dark theme designed for workshop environments |

---

## Project Structure

```
snartlaser/               Main Python package
├── __init__.py           Package metadata (version, author)
├── __main__.py           Entry point: python -m snartlaser
├── app.py                QApplication subclass – wires subsystems together
│
├── core/                 Shared infrastructure
│   ├── config.py         Persistent settings (QSettings)
│   ├── event_bus.py      Application-wide Qt signals hub
│   └── types.py          Enumerations and dataclasses (CutSettings, Point, …)
│
├── canvas/               2-D design canvas
│   ├── scene.py          DesignScene (QGraphicsScene) – items, grid, tools dispatch
│   ├── view.py           DesignView (QGraphicsView) – zoom, pan, keyboard shortcuts
│   ├── items.py          DesignItem (QGraphicsPathItem) – selectable path with ID
│   └── tools.py          Drawing tools: Select, Pan, Rectangle, Ellipse, Line
│
├── job/                  Job / layer model
│   ├── layer.py          Layer & LayerList – ordered, serialisable cut layers
│   └── settings.py       JobSettings + built-in MATERIAL_LIBRARY presets
│
├── formats/              File format I/O
│   ├── svg.py            SVG import (path, rect, circle, ellipse, line, polyline)
│   │                     and export
│   └── dxf.py            DXF import (LINE, CIRCLE, ARC, ELLIPSE, LWPOLYLINE,
│                          SPLINE) and export via ezdxf
│
├── gcode/                Machine code generation
│   └── generator.py      GCodeGenerator → GRBL G-code or HPGL for vinyl cutters
│
├── device/               Machine drivers (serial)
│   ├── base.py           BaseDevice abstract interface + DeviceWorker thread
│   ├── grbl.py           GrblDevice – GRBL laser cutter (ok/error handshake)
│   └── vinyl.py          VinylDevice – HPGL vinyl cutter
│
└── ui/                   Qt widgets
    ├── mainwindow.py      MainWindow – menus, docks, theme, file/job actions
    ├── canvas_widget.py   CanvasWidget – toolbar + DesignView
    ├── layers_panel.py    LayersPanel – layer list + inline settings editor
    ├── device_panel.py    DevicePanel – connect, jog, run/pause/stop, console
    ├── job_panel.py       JobPanel – workspace dimensions, material, notes
    └── dialogs/
        ├── device_settings.py  DeviceSettingsDialog – port, baud, type
        └── material_library.py MaterialLibraryDialog – preset browser

tests/                    pytest test suite
├── conftest.py           Session-scoped QApplication fixture
├── test_core.py          Config, EventBus, types
├── test_formats.py       SVG and DXF import/export
├── test_gcode.py         G-code and HPGL generation
├── test_job.py           Layer list, JobSettings, material library
└── test_canvas.py        DesignScene, DesignItem, tools
```

---

## Architecture

```
┌─────────────────────────────────────────────┐
│                  MainWindow                  │
│  ┌──────────────┐  ┌────────┐  ┌──────────┐ │
│  │ CanvasWidget │  │Layers  │  │ Device   │ │
│  │  DesignView  │  │Panel   │  │ Panel    │ │
│  │  DesignScene │  │        │  │          │ │
│  └──────────────┘  └────────┘  └──────────┘ │
│         │               │           │        │
│         ▼               ▼           ▼        │
│     EventBus ←──────────────────────────── │
│         │                                   │
│    ┌────┴────────────────┐                  │
│    │   Core subsystems   │                  │
│    │  Config · Types     │                  │
│    └────┬───────────┬────┘                  │
│         │           │                       │
│    ┌────▼───┐  ┌────▼────┐  ┌────────────┐ │
│    │Formats │  │  Job /  │  │  GCode     │ │
│    │SVG DXF │  │ Layers  │  │ Generator  │ │
│    └────────┘  └─────────┘  └────────────┘ │
│                                    │        │
│                              ┌─────▼──────┐ │
│                              │  Device    │ │
│                              │ GRBL/HPGL  │ │
│                              └────────────┘ │
└─────────────────────────────────────────────┘
```

**Data flow for a job:**

1. User imports SVG/DXF or draws shapes on the canvas.
2. Shapes are assigned to cut layers.
3. Each layer has `CutSettings` (speed, power, mode, passes).
4. `GCodeGenerator` converts paths + settings → G-code or HPGL.
5. Device driver sends the code over serial to the machine.

---

## Requirements

* Python ≥ 3.10
* PyQt6 ≥ 6.4
* ezdxf ≥ 1.0
* lxml ≥ 4.9
* numpy ≥ 1.24
* pyserial ≥ 3.5

---

## Installation

```bash
# Clone
git clone https://github.com/larsnygard/SnartLaser.git
cd SnartLaser

# Create a virtual environment (recommended)
python -m venv .venv
source .venv/bin/activate          # Linux/macOS
# .venv\Scripts\activate           # Windows

# Install runtime dependencies
pip install -r requirements.txt

# Install in development mode (includes dev tools)
pip install -e ".[dev]"
```

### Linux: system Qt libraries (optional)

On most distros you can use the system Qt6 instead of the bundled wheel:

```bash
# Debian/Ubuntu
sudo apt install python3-pyqt6

# Fedora
sudo dnf install python3-qt6
```

---

## Running

```bash
# As a module
python -m snartlaser

# Or via the installed console script
snartlaser
```

---

## Usage

### Drawing
1. Select a drawing tool from the toolbar (Rectangle **R**, Ellipse **E**, Line **L**).
2. Click and drag on the canvas to create shapes.
3. Use the **Select** tool (**S**) to move or resize items.
4. Press **Delete** to remove selected items.

### Layers
1. Open the **Layers** panel (right sidebar).
2. Click **+** to add a layer or **Preset…** to use a material preset.
3. Assign shapes to a layer by right-clicking (future) or by colour matching.
4. Edit speed, power, passes and mode for each layer.

### Connecting a Machine
1. Open **Machine → Connect…** (or click **Settings…** in the Device panel).
2. Select the device type (GRBL Laser / Vinyl Cutter), serial port and baud rate.
3. Click **OK**, then **Connect** in the Device panel.

### Running a Job
1. Set up your layers and shapes.
2. Click **▶ Run** (or press **F5**).
3. Monitor progress in the Device panel console.

---

## Development

```bash
# Run tests (headless – uses offscreen Qt platform)
pytest

# Run a specific test file
pytest tests/test_gcode.py -v

# Install dev dependencies only
pip install -r requirements-dev.txt
```

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| S | Select tool |
| H | Pan tool |
| R | Rectangle tool |
| E | Ellipse tool |
| L | Line tool |
| Ctrl+= | Zoom in |
| Ctrl+- | Zoom out |
| Ctrl+0 | Fit to window |
| Ctrl+A | Select all |
| Delete | Delete selected |
| F5 | Run job |
| Ctrl+N | New file |
| Ctrl+O | Open file |
| Ctrl+S | Save file |
| Ctrl+Q | Quit |

---

## Supported Machines

| Machine type | Protocol | Tested firmware |
|---|---|---|
| CO₂ / diode laser cutter | GRBL G-code over USB-serial | GRBL 1.1, grblHAL |
| CNC router (spindle mode) | GRBL G-code | GRBL 1.1 |
| Vinyl cutter | HPGL over USB-serial | Roland, Graphtec, Silhouette |
| 3-D printer (laser head) | Marlin G-code | Marlin 2.x |

---

## License

MIT – see [LICENSE](LICENSE).
