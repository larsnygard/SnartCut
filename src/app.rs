//! Top-level Iced application state and message handler.

use std::path::PathBuf;

use iced::widget::{
    button, canvas, column, container, row, scrollable, text, tooltip,
};
use iced::{
    Alignment, Color, Element, Length, Subscription, Task, Theme, Vector,
};
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

use crate::canvas::scene::Scene;
use crate::canvas::tools::ToolState;
use crate::core::config::Config;
use crate::core::types::{CutSettings, DeviceType, PathData, ToolType};
use crate::device::base::{DeviceCommand, DeviceEvent};
use crate::formats::{dxf, svg};
use crate::gcode::generator::GCodeGenerator;
use crate::job::layer::LayerList;
use crate::job::settings::{material_library, JobSettings};
use crate::ui::canvas_widget::{CanvasState, DesignCanvas};
use crate::ui::device_panel;
use crate::ui::dialogs::device_settings::{DeviceSettingsState, device_settings_view};
use crate::ui::dialogs::material_library::material_library_view;
use crate::ui::job_panel;
use crate::ui::layers_panel;

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Message {
    // ---- File / project ----
    NewFile,
    OpenFile,
    FileOpened(Option<PathBuf>),
    FileParsed(Result<(Scene, JobSettings), String>),
    SaveFile,
    SaveFileAs,
    FileSavePathChosen(Option<PathBuf>),
    ImportSvg,
    ImportSvgPathChosen(Option<PathBuf>),
    ImportDxf,
    ImportDxfPathChosen(Option<PathBuf>),
    ExportGcode,
    GcodePathChosen(Option<PathBuf>),

    // ---- Canvas ----
    ToolSelected(ToolType),
    ZoomIn,
    ZoomOut,
    ZoomReset,
    PanCanvas(f32, f32),
    CursorMoved(f64, f64),
    AddPath(PathData, String),
    SelectItem(Uuid),
    DeselectAll,
    SelectRect(f64, f64, f64, f64),
    TranslateSelected(f64, f64),
    DeleteSelected,

    // ---- Layers ----
    AddLayer,
    RemoveLayer(usize),
    SelectLayer(usize),
    LayerNameChanged(usize, String),
    LayerSpeedChanged(usize, f64),
    LayerPowerChanged(usize, f64),
    LayerPassesChanged(usize, u32),
    LayerEnabledToggled(usize, bool),

    // ---- Job ----
    WorkspaceWidthChanged(f64),
    WorkspaceHeightChanged(f64),
    MaterialNoteChanged(String),
    JobNotesChanged(String),
    RightTabSelected(RightTab),

    // ---- Device ----
    ConnectDevice,
    DisconnectDevice,
    SendJob,
    PauseJob,
    CancelJob,
    HomeDevice,
    JogDevice(f64, f64),
    DeviceEvent(DeviceEvent),

    // ---- Dialogs ----
    OpenDeviceSettings,
    DevicePortChanged(String),
    DeviceBaudChanged(u32),
    DeviceTypeChanged(DeviceType),
    DeviceSettingsOk,
    OpenMaterialLibrary,
    MaterialPresetSelected(String),
    MaterialPresetApply(String),
    CloseDialog,
}

// ---------------------------------------------------------------------------
// Modal state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Modal {
    DeviceSettings(DeviceSettingsState),
    MaterialLibrary { selected: Option<String> },
}

// ---------------------------------------------------------------------------
// Right-panel tab
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RightTab {
    #[default]
    Layers,
    Job,
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

pub struct SnartLaserApp {
    pub config: Config,
    pub scene: Scene,
    pub job: JobSettings,

    pub active_tool: ToolType,
    pub active_color: String,
    pub zoom: f32,
    pub pan: Vector,
    pub cursor_pos: (f64, f64),

    pub selected_layer: Option<usize>,
    pub right_tab: RightTab,

    pub device_cmd_tx: Option<Sender<DeviceCommand>>,
    pub device_event_rx: Option<Receiver<DeviceEvent>>,
    pub device_connected: bool,
    pub device_position: (f64, f64),
    pub job_progress: Option<u8>,
    pub device_log: Vec<String>,

    pub modal: Option<Modal>,
    pub current_file: Option<PathBuf>,
    pub modified: bool,

    pub status: String,
}

impl SnartLaserApp {
    pub fn new() -> (Self, Task<Message>) {
        let config = Config::load();
        let mut job = JobSettings::new();
        job.workspace = config.workspace.clone();

        // Start with one default layer
        job.layers.add_new();

        (
            Self {
                config,
                scene: Scene::new(),
                job,
                active_tool: ToolType::Select,
                active_color: "#ff0000".to_owned(),
                zoom: 1.5,
                pan: Vector::new(20.0, 20.0),
                cursor_pos: (0.0, 0.0),
                selected_layer: Some(0),
                right_tab: RightTab::Layers,
                device_cmd_tx: None,
                device_event_rx: None,
                device_connected: false,
                device_position: (0.0, 0.0),
                job_progress: None,
                device_log: Vec::new(),
                modal: None,
                current_file: None,
                modified: false,
                status: "Ready".to_owned(),
            },
            Task::none(),
        )
    }

    // ------------------------------------------------------------------
    // Update
    // ------------------------------------------------------------------

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // ---- File ----
            Message::NewFile => {
                self.scene.clear();
                self.job = JobSettings::new();
                self.job.workspace = self.config.workspace.clone();
                self.job.layers.add_new();
                self.selected_layer = Some(0);
                self.current_file = None;
                self.modified = false;
                self.status = "New file".to_owned();
            }

            Message::OpenFile => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("SnartLaser project", &["slp", "json"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::FileOpened,
                );
            }

            Message::FileOpened(Some(path)) => {
                return Task::perform(
                    {
                        let path = path.clone();
                        async move {
                            let text = std::fs::read_to_string(&path)
                                .map_err(|e| e.to_string())?;
                            let job: JobSettings = serde_json::from_str(&text)
                                .map_err(|e| e.to_string())?;
                            let scene: Scene = serde_json::from_str(&text)
                                .unwrap_or_default();
                            Ok((scene, job))
                        }
                    },
                    Message::FileParsed,
                );
            }

            Message::FileParsed(Ok((scene, job))) => {
                self.scene = scene;
                self.job = job;
                self.modified = false;
                self.status = "File opened".to_owned();
            }

            Message::FileParsed(Err(e)) => {
                self.status = format!("Open error: {e}");
            }

            Message::SaveFile => {
                if let Some(path) = self.current_file.clone() {
                    self.save_to(&path);
                } else {
                    return self.update(Message::SaveFileAs);
                }
            }

            Message::SaveFileAs => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("SnartLaser project", &["slp"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::FileSavePathChosen,
                );
            }

            Message::FileSavePathChosen(Some(path)) => {
                self.save_to(&path);
                self.current_file = Some(path);
            }

            Message::ImportSvg => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("SVG", &["svg"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::ImportSvgPathChosen,
                );
            }

            Message::ImportSvgPathChosen(Some(path)) => {
                match svg::load(&path) {
                    Ok(pairs) => {
                        for (pd, color) in pairs {
                            self.scene.add_path(pd, &color);
                        }
                        self.modified = true;
                        self.status = format!(
                            "Imported SVG: {}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        );
                    }
                    Err(e) => self.status = format!("SVG import error: {e}"),
                }
            }

            Message::ImportDxf => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("DXF", &["dxf"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::ImportDxfPathChosen,
                );
            }

            Message::ImportDxfPathChosen(Some(path)) => {
                match dxf::load(&path) {
                    Ok(pairs) => {
                        for (pd, color) in pairs {
                            self.scene.add_path(pd, &color);
                        }
                        self.modified = true;
                        self.status = format!(
                            "Imported DXF: {}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        );
                    }
                    Err(e) => self.status = format!("DXF import error: {e}"),
                }
            }

            Message::ExportGcode => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("G-code", &["gcode", "nc", "txt"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::GcodePathChosen,
                );
            }

            Message::GcodePathChosen(Some(path)) => {
                self.export_gcode(&path);
            }

            // ---- Canvas ----
            Message::ToolSelected(t) => {
                self.active_tool = t;
            }

            Message::ZoomIn => {
                self.zoom = (self.zoom * 1.25).min(50.0);
            }
            Message::ZoomOut => {
                self.zoom = (self.zoom / 1.25).max(0.05);
            }
            Message::ZoomReset => {
                self.zoom = 1.5;
                self.pan = Vector::new(20.0, 20.0);
            }

            Message::PanCanvas(dx, dy) => {
                self.pan.x += dx;
                self.pan.y += dy;
            }

            Message::CursorMoved(x, y) => {
                self.cursor_pos = (x, y);
            }

            Message::AddPath(path, color) => {
                let id = self.scene.add_path(path, &color);
                // Assign to active layer
                if let Some(idx) = self.selected_layer {
                    if let Some(layer) = self.job.layers.get_mut(idx) {
                        layer.add_item(id);
                    }
                }
                self.modified = true;
            }

            Message::SelectItem(id) => {
                self.scene.set_selection(vec![id]);
            }

            Message::DeselectAll => {
                self.scene.deselect_all();
            }

            Message::SelectRect(rx, ry, rw, rh) => {
                let ids = self.scene.items_in_rect(rx, ry, rw, rh);
                self.scene.set_selection(ids);
            }

            Message::TranslateSelected(dx, dy) => {
                self.scene.translate_selected(dx, dy);
                self.modified = true;
            }

            Message::DeleteSelected => {
                self.scene.remove_selected();
                self.modified = true;
            }

            // ---- Layers ----
            Message::AddLayer => {
                let idx = self.job.layers.add_new();
                self.selected_layer = Some(idx);
            }

            Message::RemoveLayer(idx) => {
                self.job.layers.remove(idx);
                self.selected_layer = if self.job.layers.is_empty() {
                    None
                } else {
                    Some(idx.saturating_sub(1))
                };
            }

            Message::SelectLayer(idx) => {
                self.selected_layer = Some(idx);
                // Change active colour to match layer
                if let Some(layer) = self.job.layers.get(idx) {
                    self.active_color = layer.color().to_owned();
                }
            }

            Message::LayerNameChanged(idx, name) => {
                if let Some(l) = self.job.layers.get_mut(idx) {
                    l.settings.name = name;
                }
            }

            Message::LayerSpeedChanged(idx, val) => {
                if let Some(l) = self.job.layers.get_mut(idx) {
                    l.settings.speed_mm_s = val;
                }
            }

            Message::LayerPowerChanged(idx, val) => {
                if let Some(l) = self.job.layers.get_mut(idx) {
                    l.settings.power_pct = val.clamp(0.0, 100.0);
                }
            }

            Message::LayerPassesChanged(idx, val) => {
                if let Some(l) = self.job.layers.get_mut(idx) {
                    l.settings.passes = val.max(1);
                }
            }

            Message::LayerEnabledToggled(idx, val) => {
                if let Some(l) = self.job.layers.get_mut(idx) {
                    l.settings.enabled = val;
                }
            }

            // ---- Job ----
            Message::WorkspaceWidthChanged(v) => {
                self.job.workspace.width_mm = v;
                self.config.workspace.width_mm = v;
            }
            Message::WorkspaceHeightChanged(v) => {
                self.job.workspace.height_mm = v;
                self.config.workspace.height_mm = v;
            }
            Message::MaterialNoteChanged(v) => self.job.material = v,
            Message::JobNotesChanged(v) => self.job.notes = v,
            Message::RightTabSelected(tab) => self.right_tab = tab,

            // ---- Device ----
            Message::ConnectDevice => {
                let (cmd_tx, event_rx) = match self.config.device.device_type {
                    DeviceType::VinylCutter => crate::device::vinyl::spawn(),
                    _ => crate::device::grbl::spawn(),
                };
                let port = self.config.device.port.clone();
                let baud = self.config.device.baud_rate;
                let _ = cmd_tx.try_send(DeviceCommand::Connect { port, baud_rate: baud });
                self.device_cmd_tx = Some(cmd_tx);
                self.device_event_rx = Some(event_rx);
            }

            Message::DisconnectDevice => {
                if let Some(tx) = &self.device_cmd_tx {
                    let _ = tx.try_send(DeviceCommand::Disconnect);
                }
                self.device_cmd_tx = None;
                self.device_connected = false;
            }

            Message::SendJob => {
                if let Some(tx) = &self.device_cmd_tx {
                    let lines = self.build_job_lines();
                    let _ = tx.try_send(DeviceCommand::SendJob(lines));
                }
            }

            Message::PauseJob => {
                if let Some(tx) = &self.device_cmd_tx {
                    let _ = tx.try_send(DeviceCommand::FeedHold);
                }
            }

            Message::CancelJob => {
                if let Some(tx) = &self.device_cmd_tx {
                    let _ = tx.try_send(DeviceCommand::SoftReset);
                }
                self.job_progress = None;
            }

            Message::HomeDevice => {
                if let Some(tx) = &self.device_cmd_tx {
                    let _ = tx.try_send(DeviceCommand::Home);
                }
            }

            Message::JogDevice(x, y) => {
                if let Some(tx) = &self.device_cmd_tx {
                    let _ = tx.try_send(DeviceCommand::Jog {
                        x,
                        y,
                        feed_mm_min: 3000.0,
                    });
                }
            }

            Message::DeviceEvent(event) => match event {
                DeviceEvent::Connected => {
                    self.device_connected = true;
                    self.device_log.push("Connected".to_owned());
                }
                DeviceEvent::Disconnected => {
                    self.device_connected = false;
                    self.device_log.push("Disconnected".to_owned());
                }
                DeviceEvent::LineReceived(line) => {
                    self.device_log.push(line);
                    if self.device_log.len() > 500 {
                        self.device_log.drain(..100);
                    }
                }
                DeviceEvent::PositionUpdate(x, y) => {
                    self.device_position = (x, y);
                }
                DeviceEvent::JobProgress(pct) => {
                    self.job_progress = Some(pct);
                }
                DeviceEvent::JobFinished(ok) => {
                    self.job_progress = None;
                    self.status = if ok {
                        "Job finished successfully".to_owned()
                    } else {
                        "Job finished with errors".to_owned()
                    };
                }
                DeviceEvent::Message(msg) => {
                    self.device_log.push(msg.clone());
                    self.status = msg;
                }
            },

            // ---- Dialogs ----
            Message::OpenDeviceSettings => {
                self.modal = Some(Modal::DeviceSettings(
                    DeviceSettingsState::from_config(&self.config.device),
                ));
            }

            Message::DevicePortChanged(port) => {
                if let Some(Modal::DeviceSettings(ref mut s)) = self.modal {
                    s.port = port.clone();
                }
                self.config.device.port = port;
            }

            Message::DeviceBaudChanged(baud) => {
                if let Some(Modal::DeviceSettings(ref mut s)) = self.modal {
                    s.baud_rate = baud.to_string();
                }
                self.config.device.baud_rate = baud;
            }

            Message::DeviceTypeChanged(dt) => {
                if let Some(Modal::DeviceSettings(ref mut s)) = self.modal {
                    s.device_type = dt;
                }
                self.config.device.device_type = dt;
            }

            Message::DeviceSettingsOk => {
                self.config.save();
                self.modal = None;
            }

            Message::OpenMaterialLibrary => {
                self.modal = Some(Modal::MaterialLibrary { selected: None });
            }

            Message::MaterialPresetSelected(name) => {
                if let Some(Modal::MaterialLibrary { ref mut selected }) = self.modal {
                    *selected = Some(name);
                }
            }

            Message::MaterialPresetApply(name) => {
                let lib = material_library();
                if let Some(preset) = lib.get(name.as_str()) {
                    if let Some(idx) = self.selected_layer {
                        if let Some(layer) = self.job.layers.get_mut(idx) {
                            layer.settings = preset.clone();
                        }
                    }
                }
                self.modal = None;
            }

            Message::CloseDialog => {
                self.modal = None;
            }

            // Catch-all for dialog-cancelled file pickers
            Message::FileOpened(None)
            | Message::FileSavePathChosen(None)
            | Message::ImportSvgPathChosen(None)
            | Message::ImportDxfPathChosen(None)
            | Message::GcodePathChosen(None) => {}
        }

        Task::none()
    }

    // ------------------------------------------------------------------
    // View
    // ------------------------------------------------------------------

    pub fn view(&self) -> Element<'_, Message> {
        let main_content = self.main_layout();

        // Overlay modal if open
        if let Some(modal) = &self.modal {
            let overlay: Element<_> = match modal {
                Modal::DeviceSettings(state) => device_settings_view(state),
                Modal::MaterialLibrary { selected } => {
                    material_library_view(selected.as_deref())
                }
            };
            // Stack overlay on top
            return iced::widget::stack![main_content, overlay].into();
        }

        main_content
    }

    fn main_layout(&self) -> Element<'_, Message> {
        // ---- Toolbar (left side) ----
        let toolbar = self.build_toolbar();

        // ---- Canvas ----
        let canvas_widget = canvas(DesignCanvas {
            scene: &self.scene,
            active_tool: self.active_tool,
            active_color: &self.active_color,
            zoom: self.zoom,
            pan: self.pan,
            workspace_w: self.job.workspace.width_mm,
            workspace_h: self.job.workspace.height_mm,
            show_grid: self.job.workspace.show_grid,
            grid_spacing: self.job.workspace.grid_spacing_mm,
        })
        .width(Length::Fill)
        .height(Length::Fill);

        // ---- Right panel ----
        let right_panel = self.build_right_panel();

        // ---- Status bar ----
        let status = container(
            row![
                text(&self.status).size(12).style(|_: &iced::Theme| text::Style {
                    color: Some(Color::from_rgb(0.7, 0.7, 0.7)),
                }),
                iced::widget::horizontal_space(),
                text(format!(
                    "X: {:.3}  Y: {:.3}",
                    self.cursor_pos.0, self.cursor_pos.1
                ))
                .size(12)
                .style(|_: &iced::Theme| text::Style {
                    color: Some(Color::from_rgb(0.7, 0.7, 0.7)),
                }),
                text(format!("  Zoom: {:.0}%", self.zoom * 100.0))
                    .size(12)
                    .style(|_: &iced::Theme| text::Style {
                        color: Some(Color::from_rgb(0.7, 0.7, 0.7)),
                    }),
            ]
            .spacing(8)
            .padding(iced::Padding::from([2, 8])),
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.08, 0.08, 0.08))),
            ..Default::default()
        })
        .width(Length::Fill);

        // ---- Menu bar ----
        let menu = self.build_menu_bar();

        // ---- Device panel (bottom) ----
        let device_panel = device_panel::device_view(
            &self.config,
            self.device_connected,
            self.device_position,
            self.job_progress,
            &self.device_log,
        );

        let center_col = column![canvas_widget, device_panel].spacing(0);

        let main_row = row![
            toolbar,
            center_col,
            right_panel,
        ]
        .spacing(0);

        column![menu, main_row, status]
            .spacing(0)
            .into()
    }

    fn build_menu_bar(&self) -> Element<'_, Message> {
        let btn = |label: &'static str, msg: Message| {
            button(text(label).size(13))
                .on_press(msg)
                .style(|_t, _s| button::Style {
                    background: None,
                    text_color: Color::WHITE,
                    ..Default::default()
                })
        };

        container(
            row![
                btn("New", Message::NewFile),
                btn("Open…", Message::OpenFile),
                btn("Save", Message::SaveFile),
                btn("Save As…", Message::SaveFileAs),
                text("  |  ").size(13).style(|_: &iced::Theme| text::Style {
                    color: Some(Color::from_rgb(0.4, 0.4, 0.4)),
                }),
                btn("Import SVG…", Message::ImportSvg),
                btn("Import DXF…", Message::ImportDxf),
                btn("Export G-code…", Message::ExportGcode),
                text("  |  ").size(13).style(|_: &iced::Theme| text::Style {
                    color: Some(Color::from_rgb(0.4, 0.4, 0.4)),
                }),
                btn("–", Message::ZoomOut),
                btn("+", Message::ZoomIn),
                btn("1:1", Message::ZoomReset),
            ]
            .spacing(2)
            .padding(iced::Padding::from([4, 8]))
            .align_y(Alignment::Center),
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.13, 0.13, 0.13))),
            border: iced::Border {
                color: Color::from_rgb(0.25, 0.25, 0.25),
                width: 0.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .width(Length::Fill)
        .into()
    }

    fn build_toolbar(&self) -> Element<'_, Message> {
        let tools = ToolType::all()
            .iter()
            .fold(column![].spacing(2).padding(4), |col, &tool| {
                let is_active = self.active_tool == tool;
                col.push(
                    tooltip(
                        button(text(tool_icon(tool)).size(18))
                            .on_press(Message::ToolSelected(tool))
                            .style(move |_t, _s| button::Style {
                                background: Some(iced::Background::Color(if is_active {
                                    Color::from_rgba(0.0, 0.47, 0.83, 0.6)
                                } else {
                                    Color::from_rgba(1.0, 1.0, 1.0, 0.05)
                                })),
                                text_color: Color::WHITE,
                                border: iced::Border { radius: 4.0.into(), ..Default::default() },
                                ..Default::default()
                            })
                            .width(Length::Fixed(40.0))
                            .height(Length::Fixed(40.0)),
                        container(
                            text(tool.label()).size(11).style(|_: &iced::Theme| text::Style {
                                color: Some(Color::WHITE),
                            }),
                        )
                        .padding(4)
                        .style(|_| container::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(
                                0.2, 0.2, 0.2,
                            ))),
                            ..Default::default()
                        }),
                        tooltip::Position::Right,
                    ),
                )
            });

        container(tools)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.12, 0.12, 0.12))),
                border: iced::Border {
                    color: Color::from_rgb(0.25, 0.25, 0.25),
                    width: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            })
            .width(Length::Fixed(50.0))
            .height(Length::Fill)
            .into()
    }

    fn build_right_panel(&self) -> Element<'_, Message> {
        // Tab header
        let tab_row = row![
            tab_button("Layers", self.right_tab == RightTab::Layers, Message::RightTabSelected(RightTab::Layers)),
            tab_button("Job", self.right_tab == RightTab::Job, Message::RightTabSelected(RightTab::Job)),
        ]
        .spacing(0);

        let panel_content: Element<_> = match self.right_tab {
            RightTab::Layers => {
                layers_panel::layers_view(&self.job.layers, self.selected_layer)
            }
            RightTab::Job => job_panel::job_view(&self.job),
        };

        container(
            column![tab_row, panel_content].spacing(0),
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.12, 0.12, 0.12))),
            border: iced::Border {
                color: Color::from_rgb(0.25, 0.25, 0.25),
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .width(Length::Fixed(280.0))
        .height(Length::Fill)
        .into()
    }

    // ------------------------------------------------------------------
    // Subscription – receive device events
    // ------------------------------------------------------------------

    pub fn subscription(&self) -> Subscription<Message> {
        // Device event polling is handled via Task::perform in ConnectDevice.
        // No persistent subscription needed for now.
        Subscription::none()
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    fn save_to(&mut self, path: &PathBuf) {
        // Save scene + job as a combined JSON object
        #[derive(serde::Serialize)]
        struct SaveData<'a> {
            scene: &'a Scene,
            job: &'a JobSettings,
        }
        let data = SaveData { scene: &self.scene, job: &self.job };
        match serde_json::to_string_pretty(&data) {
            Ok(json) => match std::fs::write(path, json) {
                Ok(()) => {
                    self.modified = false;
                    self.status = format!(
                        "Saved: {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    );
                    self.config.add_recent_file(&path.to_string_lossy());
                    self.config.save();
                }
                Err(e) => self.status = format!("Save error: {e}"),
            },
            Err(e) => self.status = format!("Serialise error: {e}"),
        }
    }

    fn build_job_lines(&self) -> Vec<String> {
        let gen = GCodeGenerator::new(
            self.config.device.device_type,
            self.job.workspace.height_mm,
        );

        let layer_data: Vec<_> = self
            .job
            .layers
            .iter()
            .map(|layer| {
                let paths: Vec<PathData> = layer
                    .item_ids
                    .iter()
                    .filter_map(|id| self.scene.item(*id))
                    .map(|item| item.path.clone())
                    .collect();
                (paths, layer.settings.clone())
            })
            .collect();

        let refs: Vec<_> = layer_data
            .iter()
            .map(|(paths, settings)| (paths.as_slice(), settings))
            .collect();

        gen.generate(&refs)
    }

    fn export_gcode(&mut self, path: &PathBuf) {
        let lines = self.build_job_lines();
        match std::fs::write(path, lines.join("\n")) {
            Ok(()) => {
                self.status = format!(
                    "Exported G-code: {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
            }
            Err(e) => self.status = format!("Export error: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// UI helpers
// ---------------------------------------------------------------------------

fn tool_icon(tool: ToolType) -> &'static str {
    match tool {
        ToolType::Select => "↖",
        ToolType::Pan => "✋",
        ToolType::Rectangle => "▭",
        ToolType::Ellipse => "⬭",
        ToolType::Line => "╱",
        ToolType::Polyline => "⌇",
        ToolType::Bezier => "⌢",
        _ => "?",
    }
}

fn tab_button(label: &str, active: bool, msg: Message) -> Element<'_, Message> {
    button(
        text(label).size(13).style(move |_: &iced::Theme| text::Style {
            color: Some(Color::WHITE),
        }),
    )
    .on_press(msg)
    .style(move |_t, _s| button::Style {
        background: Some(iced::Background::Color(if active {
            Color::from_rgb(0.18, 0.18, 0.18)
        } else {
            Color::from_rgb(0.12, 0.12, 0.12)
        })),
        text_color: Color::WHITE,
        border: iced::Border {
            radius: 0.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}
