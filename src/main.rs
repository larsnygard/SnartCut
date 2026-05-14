mod app;
mod canvas;
mod core;
mod device;
mod formats;
mod gcode;
mod job;
mod ui;

use app::SnartLaserApp;

fn main() -> iced::Result {
    env_logger::init();

    iced::application("SnartLaser", SnartLaserApp::update, SnartLaserApp::view)
        .theme(SnartLaserApp::theme)
        .subscription(SnartLaserApp::subscription)
        .run_with(SnartLaserApp::new)
}
