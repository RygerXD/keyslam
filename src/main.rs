#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod game;
mod localization;
mod platform;
mod render;
mod responses;
mod settings;

use app::{KeySlamApp, display_configs};
use eframe::egui;

fn main() -> eframe::Result {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(parent_process_id) = args.iter().find_map(|argument| {
        argument
            .strip_prefix(platform::HELPER_ARGUMENT_PREFIX)
            .and_then(|value| value.parse::<u32>().ok())
    }) {
        if let Err(error) = platform::run_keyboard_guard_helper(parent_process_id) {
            eprintln!("Windows keyboard helper failed: {error}");
        }
        return Ok(());
    }
    let windowed = args.iter().any(|argument| argument == "--windowed");

    let instance = match single_instance::SingleInstance::new("KeySlamSingleInstance") {
        Ok(instance) => instance,
        Err(error) => {
            eprintln!("Could not establish single-instance protection: {error}");
            return Ok(());
        }
    };
    if !instance.is_single() {
        eprintln!("KeySlam is already running.");
        return Ok(());
    }

    let displays = display_configs(windowed);
    let root_viewport = displays
        .first()
        .map_or_else(egui::ViewportBuilder::default, app::DisplayConfig::viewport)
        .with_icon(load_icon());
    let native_options = eframe::NativeOptions {
        viewport: root_viewport,
        ..Default::default()
    };
    eframe::run_native(
        "KeySlam",
        native_options,
        Box::new(move |creation_context| {
            Ok(Box::new(KeySlamApp::new(
                displays,
                &creation_context.egui_ctx,
            )))
        }),
    )
}

fn load_icon() -> std::sync::Arc<egui::IconData> {
    match image::load_from_memory(include_bytes!("../assets/keyslam.png")) {
        Ok(image) => {
            let image = image.into_rgba8();
            let width = image.width();
            let height = image.height();
            std::sync::Arc::new(egui::IconData {
                rgba: image.into_raw(),
                width,
                height,
            })
        }
        Err(_) => std::sync::Arc::new(egui::IconData::default()),
    }
}
