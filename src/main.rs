#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod gui;
mod html_parser;

fn main() {
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| Box::new(gui::TeleTextApp::new(cc))),
    );
}
