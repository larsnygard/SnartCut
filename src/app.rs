//! Top-level Iced application state and message handler.

use std::path::PathBuf;

use iced::widget::{button, canvas, column, container, row, text, tooltip};
use iced::{
    Alignment, Color, Element, Length, Subscription, Task, Theme, Vector,
};
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

use crate::canvas::scene::Scene;
use crate::core::config::{BindingId, Config, DeviceProfile, ScrollAction};
use crate::core::types::{DeviceType, PathData, ToolType};
use crate::device::base::{DeviceCommand, DeviceEvent};
use crate::formats::{dxf, svg};
use crate::gcode::generator::GCodeGenerator;
use crate::job::settings::{material_library, JobSettings};
use crate::ui::canvas_widget::DesignCanvas;
use crate::ui::device_panel;
use crate::ui::dialogs::device_settings::{DeviceSettingsState, device_settings_view};
use crate::ui::dialogs::material_library::material_library_view;
use crate::ui::dialogs::preferences::{preferences_view, PrefTab, PreferencesState};
use crate::ui::job_panel;
use crate::ui::layers_panel;
use crate::ui::ruler;

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
    KeyPressed(iced::keyboard::Key),
    KeyReleased(iced::keyboard::Key),
    ScrollCanvas(f32, f32, f32),  // delta, cursor_x, cursor_y
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

    // ---- Menus ----
    ToggleMenu(MenuId),
    CloseMenu,

    // ---- Dialogs ----
    OpenDeviceSettings,
    DevicePortChanged(String),
    DeviceBaudChanged(u32),
    DeviceTypeChanged(DeviceType),
    DeviceProfileSelected(usize),
    DeviceProfileNew,
    DeviceProfileNameChanged(String),
    DeviceProfileDelete,
    DeviceProfileWorkAreaW(f64),
    DeviceProfileWorkAreaH(f64),
    DeviceSettingsOk,
    OpenMaterialLibrary,
    MaterialPresetSelected(String),
    MaterialPresetApply(String),
    CloseDialog,
    CloseModal,

    // ---- Preferences ----
    OpenPreferences,
    PrefTabSelected(PrefTab),
    PrefBeginRebind(BindingId),
    PrefClearBinding(BindingId),
    PrefScrollChanged(ScrollAction),
    PrefCanvasBgChanged(String),
    PrefWorkspaceBgChanged(String),
    PrefGridColorChanged(String),
    PrefGridOpacityChanged(f32),
    PrefShapeStrokeChanged(f32),
    PrefSelectionColorChanged(String),
    PrefPreviewColorChanged(String),
    PrefAntialiasingChanged(bool),
    PrefSave,
}

// ---------------------------------------------------------------------------
// Menu state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuId {
    File,
    View,
}

// ---------------------------------------------------------------------------
// Modal state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Modal {
    DeviceSettings(DeviceSettingsState),
    MaterialLibrary { selected: Option<String> },
    Preferences(PreferencesState),
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

pub struct SnartCutApp {
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

    pub pre_space_tool: Option<ToolType>,
    pub canvas_revision: u64,
    pub modal: Option<Modal>,
    pub open_menu: Option<MenuId>,
    pub current_file: Option<PathBuf>,
    pub modified: bool,

    pub status: String,
}

impl SnartCutApp {
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
                pre_space_tool: None,
                canvas_revision: 0,
                modal: None,
                open_menu: None,
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
        // Close any open dropdown on every message except ToggleMenu itself.
        if !matches!(message, Message::ToggleMenu(_)) {
            self.open_menu = None;
        }

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
                            .add_filter("SnartCut project", &["slp", "json"])
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
                            .add_filter("SnartCut project", &["slp"])
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

            Message::KeyPressed(key) => {
                // While preferences dialog is open and capturing a rebind, grab next key.
                if let Some(Modal::Preferences(ref mut s)) = self.modal {
                    if s.capturing.is_some() {
                        use iced::keyboard::key::Named;
                        let s_str = key_to_string(&key);
                        // Escape cancels; modifiers are ignored; any other key sets the binding.
                        match &key {
                            iced::keyboard::Key::Named(Named::Escape) => {
                                s.capturing = None;
                            }
                            _ if s_str.is_empty() => {} // unidentified / modifier-only
                            _ => {
                                let id = s.capturing.take().unwrap();
                                self.config.bindings.set(id, s_str);
                            }
                        }
                        return Task::none();
                    }
                }
                // Don't fire shortcuts while any modal is open.
                if self.modal.is_some() {
                    return Task::none();
                }
                let b = self.config.bindings.clone();
                if key_matches(&key, &b.temp_pan) {
                    if self.pre_space_tool.is_none() {
                        self.pre_space_tool = Some(self.active_tool);
                        self.active_tool = ToolType::Pan;
                    }
                } else if key_matches(&key, &b.tool_select) {
                    self.active_tool = ToolType::Select;
                } else if key_matches(&key, &b.tool_pan) {
                    self.active_tool = ToolType::Pan;
                } else if key_matches(&key, &b.tool_rect) {
                    self.active_tool = ToolType::Rectangle;
                } else if key_matches(&key, &b.tool_ellipse) {
                    self.active_tool = ToolType::Ellipse;
                } else if key_matches(&key, &b.tool_line) {
                    self.active_tool = ToolType::Line;
                } else if key_matches(&key, &b.tool_polyline) {
                    self.active_tool = ToolType::Polyline;
                } else if key_matches(&key, &b.delete_selected) {
                    return Task::done(Message::DeleteSelected);
                } else if key_matches(&key, &b.zoom_in) {
                    self.zoom = (self.zoom * 1.25).min(50.0);
                    self.canvas_revision += 1;
                } else if key_matches(&key, &b.zoom_out) {
                    self.zoom = (self.zoom / 1.25).max(0.05);
                    self.canvas_revision += 1;
                } else if key_matches(&key, &b.zoom_reset) {
                    self.zoom = 1.5;
                    self.pan = Vector::new(20.0, 20.0);
                    self.canvas_revision += 1;
                }
            }

            Message::KeyReleased(key) => {
                if key_matches(&key, &self.config.bindings.temp_pan) {
                    if let Some(prev) = self.pre_space_tool.take() {
                        self.active_tool = prev;
                    }
                }
            }

            Message::ScrollCanvas(dy, cx, cy) => {
                match self.config.mouse_bindings.scroll {
                    ScrollAction::Zoom => {
                        let factor = if dy > 0.0 { 1.12f32 } else { 1.0 / 1.12 };
                        let old_zoom = self.zoom;
                        let new_zoom = (old_zoom * factor).clamp(0.05, 50.0);
                        let ratio = new_zoom / old_zoom;
                        self.pan.x = cx - (cx - self.pan.x) * ratio;
                        self.pan.y = cy - (cy - self.pan.y) * ratio;
                        self.zoom = new_zoom;
                    }
                    ScrollAction::PanVertical => {
                        self.pan.y += dy * 30.0;
                    }
                }
                self.canvas_revision += 1;
            }

            Message::ZoomIn => {
                self.zoom = (self.zoom * 1.25).min(50.0);
                self.canvas_revision += 1;
            }
            Message::ZoomOut => {
                self.zoom = (self.zoom / 1.25).max(0.05);
                self.canvas_revision += 1;
            }
            Message::ZoomReset => {
                self.zoom = 1.5;
                self.pan = Vector::new(20.0, 20.0);
                self.canvas_revision += 1;
            }

            Message::PanCanvas(dx, dy) => {
                self.pan.x += dx;
                self.pan.y += dy;
                self.canvas_revision += 1;
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
                self.canvas_revision += 1;
            }
            Message::WorkspaceHeightChanged(v) => {
                self.job.workspace.height_mm = v;
                self.config.workspace.height_mm = v;
                self.canvas_revision += 1;
            }
            Message::MaterialNoteChanged(v) => self.job.material = v,
            Message::JobNotesChanged(v) => self.job.notes = v,
            Message::RightTabSelected(tab) => self.right_tab = tab,

            // ---- Device ----
            Message::ConnectDevice => {
                let profile = self.config.device.active().clone();
                let (cmd_tx, event_rx) = match profile.device_type {
                    DeviceType::VinylCutter => crate::device::vinyl::spawn(),
                    _ => crate::device::grbl::spawn(),
                };
                let _ = cmd_tx.try_send(DeviceCommand::Connect { port: profile.port, baud_rate: profile.baud_rate });
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
                self.config.device.active_mut().port = port;
            }

            Message::DeviceBaudChanged(baud) => {
                if let Some(Modal::DeviceSettings(ref mut s)) = self.modal {
                    s.baud_rate = baud.to_string();
                }
                self.config.device.active_mut().baud_rate = baud;
            }

            Message::DeviceTypeChanged(dt) => {
                if let Some(Modal::DeviceSettings(ref mut s)) = self.modal {
                    s.device_type = dt;
                }
                self.config.device.active_mut().device_type = dt;
            }

            Message::DeviceProfileSelected(idx) => {
                if idx < self.config.device.profiles.len() {
                    self.config.device.active_profile = idx;
                    // Apply the profile's work area
                    let (w, h) = {
                        let p = self.config.device.active();
                        (p.work_area_w, p.work_area_h)
                    };
                    self.job.workspace.width_mm = w;
                    self.job.workspace.height_mm = h;
                    self.config.workspace.width_mm = w;
                    self.config.workspace.height_mm = h;
                    self.canvas_revision += 1;
                    // Refresh the dialog state if it's open
                    if let Some(Modal::DeviceSettings(ref mut s)) = self.modal {
                        *s = DeviceSettingsState::from_config(&self.config.device);
                    }
                    self.config.save();
                }
            }

            Message::DeviceProfileNew => {
                let n = self.config.device.profiles.len() + 1;
                self.config.device.profiles.push(DeviceProfile {
                    name: format!("Profile {n}"),
                    port: String::new(),
                    baud_rate: 115200,
                    device_type: DeviceType::GrblLaser,
                    work_area_w: 400.0,
                    work_area_h: 400.0,
                });
                self.config.device.active_profile = self.config.device.profiles.len() - 1;
                if let Some(Modal::DeviceSettings(ref mut s)) = self.modal {
                    *s = DeviceSettingsState::from_config(&self.config.device);
                }
                self.config.save();
            }

            Message::DeviceProfileNameChanged(name) => {
                self.config.device.active_mut().name = name.clone();
                if let Some(Modal::DeviceSettings(ref mut s)) = self.modal {
                    s.profile_name = name;
                }
            }

            Message::DeviceProfileWorkAreaW(v) => {
                let v = v.max(1.0);
                self.config.device.active_mut().work_area_w = v;
                self.job.workspace.width_mm = v;
                self.config.workspace.width_mm = v;
                self.canvas_revision += 1;
                if let Some(Modal::DeviceSettings(ref mut s)) = self.modal {
                    s.work_area_w = format!("{v:.1}");
                }
            }

            Message::DeviceProfileWorkAreaH(v) => {
                let v = v.max(1.0);
                self.config.device.active_mut().work_area_h = v;
                self.job.workspace.height_mm = v;
                self.config.workspace.height_mm = v;
                self.canvas_revision += 1;
                if let Some(Modal::DeviceSettings(ref mut s)) = self.modal {
                    s.work_area_h = format!("{v:.1}");
                }
            }

            Message::DeviceProfileDelete => {
                if self.config.device.profiles.len() > 1 {
                    let idx = self.config.device.active_profile;
                    self.config.device.profiles.remove(idx);
                    self.config.device.active_profile =
                        idx.min(self.config.device.profiles.len() - 1);
                    if let Some(Modal::DeviceSettings(ref mut s)) = self.modal {
                        *s = DeviceSettingsState::from_config(&self.config.device);
                    }
                    self.config.save();
                }
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

            Message::CloseDialog | Message::CloseModal => {
                self.modal = None;
            }

            // ---- Menus ----
            Message::ToggleMenu(id) => {
                self.open_menu = if self.open_menu == Some(id) { None } else { Some(id) };
            }

            Message::CloseMenu => { /* open_menu already cleared above */ }

            // ---- Preferences ----
            Message::OpenPreferences => {
                self.modal = Some(Modal::Preferences(PreferencesState::default()));
            }
            Message::PrefTabSelected(tab) => {
                if let Some(Modal::Preferences(ref mut s)) = self.modal {
                    s.tab = tab;
                    s.capturing = None;
                }
            }
            Message::PrefBeginRebind(id) => {
                if let Some(Modal::Preferences(ref mut s)) = self.modal {
                    s.capturing = Some(id);
                }
            }
            Message::PrefClearBinding(id) => {
                self.config.bindings.set(id, String::new());
            }
            Message::PrefScrollChanged(action) => {
                self.config.mouse_bindings.scroll = action;
            }
            Message::PrefCanvasBgChanged(v)       => self.config.visual.canvas_bg = v,
            Message::PrefWorkspaceBgChanged(v)    => self.config.visual.workspace_bg = v,
            Message::PrefGridColorChanged(v)      => self.config.visual.grid_color = v,
            Message::PrefGridOpacityChanged(v)    => self.config.visual.grid_opacity = v,
            Message::PrefShapeStrokeChanged(v)    => self.config.visual.shape_stroke_px = v,
            Message::PrefSelectionColorChanged(v) => self.config.visual.selection_color = v,
            Message::PrefPreviewColorChanged(v)   => self.config.visual.preview_color = v,
            Message::PrefAntialiasingChanged(v)   => self.config.visual.antialiasing = v,
            Message::PrefSave => {
                self.config.save();
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

        // Build a list of overlay layers to stack on top of main content.
        let mut layers: Vec<Element<_>> = vec![main_content];

        // Dropdown menu overlay
        if let Some(menu_id) = self.open_menu {
            // Full-screen transparent backdrop – clicking it closes the menu.
            let backdrop = button(iced::widget::Space::new(Length::Fill, Length::Fill))
                .on_press(Message::CloseMenu)
                .style(|_t, _s| button::Style {
                    background: None,
                    ..Default::default()
                })
                .width(Length::Fill)
                .height(Length::Fill);

            // The dropdown panel itself, positioned below the menu bar (32 px).
            let x_offset: f32 = match menu_id {
                MenuId::File => 8.0,
                MenuId::View => 60.0,
            };
            let dropdown = container(self.build_dropdown(menu_id))
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.18, 0.18, 0.18))),
                    text_color: None,
                    border: iced::Border {
                        color: Color::from_rgb(0.35, 0.35, 0.35),
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    shadow: iced::Shadow {
                        color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                        offset: iced::Vector::new(0.0, 4.0),
                        blur_radius: 8.0,
                    },
                })
                .width(Length::Shrink);

            // Wrap dropdown in a full-size container with padding to position it.
            let positioned = container(
                column![
                    iced::widget::vertical_space().height(32.0),
                    row![
                        iced::widget::horizontal_space().width(x_offset),
                        dropdown,
                    ],
                ],
            )
            .width(Length::Fill)
            .height(Length::Fill);

            layers.push(backdrop.into());
            layers.push(positioned.into());
        }

        // Modal dialog overlay
        if let Some(modal) = &self.modal {
            let dialog: Element<_> = match modal {
                Modal::DeviceSettings(state) => device_settings_view(state),
                Modal::MaterialLibrary { selected } => {
                    material_library_view(selected.as_deref())
                }
                Modal::Preferences(state) => preferences_view(
                    state.tab,
                    state.capturing,
                    &self.config.visual,
                    &self.config.bindings,
                    &self.config.mouse_bindings,
                ),
            };
            // Dim backdrop
            let backdrop = button(iced::widget::Space::new(Length::Fill, Length::Fill))
                .on_press(Message::CloseModal)
                .style(|_t, _s| button::Style {
                    background: Some(iced::Background::Color(
                        Color::from_rgba(0.0, 0.0, 0.0, 0.55),
                    )),
                    ..Default::default()
                })
                .width(Length::Fill)
                .height(Length::Fill);
            // Centered container
            let centered = container(dialog)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .width(Length::Fill)
                .height(Length::Fill);
            layers.push(backdrop.into());
            layers.push(centered.into());
        }

        iced::widget::Stack::with_children(layers).into()
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
            visual: &self.config.visual,
            canvas_revision: self.canvas_revision,
        })
        .width(Length::Fill)
        .height(Length::Fill);

        // ---- Rulers ----
        let h_ruler = ruler::h_ruler(self.zoom, self.pan.x);
        let v_ruler = ruler::v_ruler(self.zoom, self.pan.y);
        let corner  = ruler::corner();

        // Ruler row: corner square + horizontal ruler
        let ruler_row = row![
            corner,
            h_ruler,
        ]
        .spacing(0);

        // Canvas area: vertical ruler on the left, design canvas on the right
        let canvas_area = row![
            v_ruler,
            canvas_widget,
        ]
        .spacing(0)
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

        let canvas_with_rulers = column![ruler_row, canvas_area].spacing(0).height(Length::Fill);

        let center_col = column![canvas_with_rulers, device_panel].spacing(0);

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
        let menu_header = |label: &'static str, id: MenuId| {
            let active = self.open_menu == Some(id);
            button(text(label).size(13))
                .on_press(Message::ToggleMenu(id))
                .style(move |_t, _s| button::Style {
                    background: Some(iced::Background::Color(if active {
                        Color::from_rgb(0.25, 0.25, 0.25)
                    } else {
                        Color::TRANSPARENT
                    })),
                    text_color: Color::WHITE,
                    border: iced::Border { radius: 3.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .padding(iced::Padding::from([4, 10]))
        };

        container(
            row![
                menu_header("File", MenuId::File),
                menu_header("View", MenuId::View),
            ]
            .spacing(0)
            .padding(iced::Padding::from([4, 4]))
            .align_y(Alignment::Center),
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.13, 0.13, 0.13))),
            border: iced::Border {
                color: Color::from_rgb(0.25, 0.25, 0.25),
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .width(Length::Fill)
        .into()
    }

    fn build_dropdown(&self, id: MenuId) -> Element<'_, Message> {
        let items: Element<_> = match id {
            MenuId::File => column![
                menu_item("New",              "Ctrl+N", Some(Message::NewFile)),
                menu_item("Open…",            "Ctrl+O", Some(Message::OpenFile)),
                menu_separator(),
                menu_item("Save",             "Ctrl+S", Some(Message::SaveFile)),
                menu_item("Save As…",  "Ctrl+Shift+S", Some(Message::SaveFileAs)),
                menu_separator(),
                menu_item("Import SVG…",      "",       Some(Message::ImportSvg)),
                menu_item("Import DXF…",      "",       Some(Message::ImportDxf)),
                menu_separator(),
                menu_item("Export G-code…",   "",       Some(Message::ExportGcode)),
                menu_separator(),
                menu_item("Preferences…",     "",       Some(Message::OpenPreferences)),
            ]
            .spacing(0)
            .padding(4)
            .into(),

            MenuId::View => column![
                menu_item("Zoom In",   "+", Some(Message::ZoomIn)),
                menu_item("Zoom Out",  "–", Some(Message::ZoomOut)),
                menu_item("Zoom 1:1",  "0", Some(Message::ZoomReset)),
            ]
            .spacing(0)
            .padding(4)
            .into(),
        };

        container(items)
            .width(Length::Fixed(220.0))
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
        use iced::keyboard;
        Subscription::batch([
            keyboard::on_key_press(|key, _mods| Some(Message::KeyPressed(key))),
            keyboard::on_key_release(|key, _mods| Some(Message::KeyReleased(key))),
        ])
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
            self.config.device.active().device_type,
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
// Key binding helpers
// ---------------------------------------------------------------------------

/// Convert an iced keyboard key to the canonical display/config string.
/// Returns an empty string for modifier keys and unidentified keys.
fn key_to_string(key: &iced::keyboard::Key) -> String {
    use iced::keyboard::key::Named;
    match key {
        iced::keyboard::Key::Named(named) => match named {
            // Modifiers — not usable as standalone bindings
            Named::Shift | Named::Control | Named::Alt | Named::Super => String::new(),
            Named::CapsLock | Named::NumLock | Named::ScrollLock     => String::new(),
            // Named keys
            Named::Space     => "Space".to_owned(),
            Named::Delete    => "Delete".to_owned(),
            Named::Backspace => "Backspace".to_owned(),
            Named::Enter     => "Enter".to_owned(),
            Named::Tab       => "Tab".to_owned(),
            Named::Escape    => "Escape".to_owned(),
            Named::ArrowUp    => "ArrowUp".to_owned(),
            Named::ArrowDown  => "ArrowDown".to_owned(),
            Named::ArrowLeft  => "ArrowLeft".to_owned(),
            Named::ArrowRight => "ArrowRight".to_owned(),
            Named::Home      => "Home".to_owned(),
            Named::End       => "End".to_owned(),
            Named::PageUp    => "PageUp".to_owned(),
            Named::PageDown  => "PageDown".to_owned(),
            Named::Insert    => "Insert".to_owned(),
            Named::F1  => "F1".to_owned(),  Named::F2  => "F2".to_owned(),
            Named::F3  => "F3".to_owned(),  Named::F4  => "F4".to_owned(),
            Named::F5  => "F5".to_owned(),  Named::F6  => "F6".to_owned(),
            Named::F7  => "F7".to_owned(),  Named::F8  => "F8".to_owned(),
            Named::F9  => "F9".to_owned(),  Named::F10 => "F10".to_owned(),
            Named::F11 => "F11".to_owned(), Named::F12 => "F12".to_owned(),
            other      => format!("{other:?}"),
        },
        iced::keyboard::Key::Character(c) => c.to_string(),
        iced::keyboard::Key::Unidentified  => String::new(),
    }
}

/// Return `true` if `key` matches the given binding string (case-insensitive).
fn key_matches(key: &iced::keyboard::Key, binding: &str) -> bool {
    if binding.is_empty() {
        return false;
    }
    key_to_string(key).eq_ignore_ascii_case(binding)
}

// ---------------------------------------------------------------------------
// UI helpers
// ---------------------------------------------------------------------------

/// A single row inside a dropdown menu.
fn menu_item<'a>(label: &'a str, shortcut: &'a str, msg: Option<Message>) -> Element<'a, Message> {
    let content = row![
        text(label).size(13).style(|_: &iced::Theme| text::Style {
            color: Some(Color::WHITE),
        }),
        iced::widget::horizontal_space(),
        text(shortcut).size(11).style(|_: &iced::Theme| text::Style {
            color: Some(Color::from_rgb(0.55, 0.55, 0.55)),
        }),
    ]
    .align_y(Alignment::Center)
    .padding(iced::Padding::from([4, 8]));

    let mut btn = button(content)
        .width(Length::Fill)
        .style(|_t, _s| button::Style {
            background: None,
            text_color: Color::WHITE,
            ..Default::default()
        });
    if let Some(m) = msg {
        btn = btn.on_press(m);
    }
    btn.into()
}

/// A thin horizontal rule used as a visual separator inside a dropdown.
fn menu_separator<'a>() -> Element<'a, Message> {
    container(iced::widget::horizontal_rule(1).style(|_: &iced::Theme| {
        iced::widget::rule::Style {
            color: Color::from_rgb(0.3, 0.3, 0.3),
            width: 1,
            radius: 0.0.into(),
            fill_mode: iced::widget::rule::FillMode::Full,
        }
    }))
    .padding(iced::Padding::from([2, 8]))
    .width(Length::Fill)
    .into()
}

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
