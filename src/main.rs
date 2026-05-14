#![allow(dead_code)]
mod app;
mod canvas;
mod core;
mod device;
mod formats;
mod gcode;
mod job;
mod ui;

use app::SnartCutApp;

fn main() -> iced::Result {
    env_logger::init();

    iced::application("SnartCut", SnartCutApp::update, SnartCutApp::view)
        .theme(SnartCutApp::theme)
        .subscription(SnartCutApp::subscription)
        .run_with(SnartCutApp::new)
}
