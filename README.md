# SnartLaser

**Open-source laser cutter and vinyl cutter design & control application.**

SnartLaser is a cross-platform desktop application inspired by LightBurn.
It runs **natively on Linux** (and also on Windows and macOS), is written in
**Rust** using the [Iced](https://github.com/iced-rs/iced) GUI framework, and
is licensed under GPLv3.

---

## Features

| Category | Details |
|---|---|
| **Design canvas** | 2-D vector editor – rectangle, ellipse, line, polyline, bezier tools; select & move; zoom/pan |
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
src/
├── main.rs               Entry point
├── app.rs                Application state, Message enum, update/view/subscription
│
├── core/                 Shared infrastructure
│   ├── config.rs         Persistent TOML config (~/.config/snart-laser/config.toml)
│   └── types.rs          Enumerations and data types (CutSettings, PathData, …)
│
├── canvas/               2-D design canvas
│   ├── scene.rs          Scene – item collection, selection, hit-test
│   ├── items.rs          DesignItem – path with UUID, colour, transform
│   └── tools.rs          Tool state machine (Select, Pan, Rectangle, Ellipse, …)
│
├── job/                  Job / layer model
│   ├── layer.rs          Layer & LayerList – ordered, serialisable cut layers
│   └── settings.rs       JobSettings + built-in material library presets
│
├── formats/              File format I/O
│   ├── svg.rs            SVG import (via usvg) and export
│   └── dxf.rs            DXF import/export (LINE, CIRCLE, ARC, LWPOLYLINE)
│
├── gcode/                Machine code generation
│   └── generator.rs      GCodeGenerator → GRBL G-code or HPGL for vinyl cutters
│
├── device/               Machine drivers (serial, async)
│   ├── base.rs           DeviceCommand / DeviceEvent channel types
│   ├── grbl.rs           GRBL laser cutter driver (ok/error handshake)
│   └── vinyl.rs          HPGL vinyl cutter driver
│
└── ui/                   Iced widgets and view functions
    ├── canvas_widget.rs  DesignCanvas – iced canvas::Program implementation
    ├── layers_panel.rs   Layers sidebar view
    ├── device_panel.rs   Device connection, jog, job control panel
    ├── job_panel.rs      Workspace dimensions, material, notes
    └── dialogs/
        ├── device_settings.rs   Device settings modal (port, baud, type)
        └── material_library.rs  Material preset browser modal
```

---

## Architecture

The application follows the **Elm architecture** as used by Iced:

```
┌─────────────────────────────────────────────┐
│              SnartLaserApp                   │
│  ┌──────────────┐  ┌────────┐  ┌──────────┐ │
│  │DesignCanvas  │  │Layers  │  │ Device   │ │
│  │(iced canvas) │  │Panel   │  │ Panel    │ │
│  └──────────────┘  └────────┘  └──────────┘ │
│         │               │           │        │
│         └───────────────┴───────────┘        │
│                        │                     │
│                   Message enum               │
│                        │                     │
│                   update()                   │
│                        │                     │
│    ┌───────────────────┬┴──────────────────┐ │
│    │    Core / Job     │   Device drivers  │ │
│    │  Config · Scene   │  GRBL · HPGL      │ │
│    │  Layers · GCode   │  (std threads +   │ │
│    │  Formats          │   tokio channels) │ │
│    └───────────────────┴───────────────────┘ │
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

### Build dependencies

* Rust ≥ 1.75 (2021 edition)
* `pkg-config`
* `libudev-dev` (Linux, for serial port enumeration)

On Debian/Ubuntu:

```bash
sudo apt install pkg-config libudev-dev
```

### Runtime

No additional runtime dependencies — SnartLaser is a statically linked native binary.

---

## Building & Running

```bash
# Clone
git clone https://github.com/larsnygard/SnartLaser.git
cd SnartLaser

# Debug build
cargo run

# Optimised release build
cargo build --release
./target/release/snart-laser
```

---

## Usage

### Drawing

1. Select a drawing tool from the left toolbar (Rectangle, Ellipse, Line, …).
2. Click and drag on the canvas to create shapes.
3. Use the **Select** tool to move items.
4. Press **Delete** to remove selected items.

### Layers

1. Open the **Layers** tab in the right panel.
2. Click **+** to add a layer or **Preset…** to apply a material preset.
3. Edit speed, power, passes and mode for each layer.

### Connecting a Machine

1. Click **Settings…** in the Device panel.
2. Select the device type (GRBL Laser / Vinyl Cutter / Marlin), serial port and baud rate.
3. Click **OK**, then **Connect**.

### Running a Job

1. Set up your layers and shapes.
2. Click **▶ Run** in the Device panel.
3. Monitor progress in the console log.

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

GNU General Public License v3.0 — see [LICENSE](LICENSE).
