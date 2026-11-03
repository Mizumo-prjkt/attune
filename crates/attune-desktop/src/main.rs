//! Attune Desktop — native Rust egui frontend for Windows and Linux.
//!
//! Replaces the Flutter desktop shell (Flutter remains the Android frontend).
//! Drives `attune-core` directly in-process; no C-ABI bridge is required.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod config;
mod fonts;
mod library;
mod master_fx_ui;
mod player;
mod smart_shuffle;
mod theme;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Must run before any thread is spawned (sets ATTUNE_CACHE_DIR for artwork extraction).
    match config::ensure_cache_dir_env() {
        Some(dir) => log::info!("Attune cache directory: {}", dir.display()),
        None => log::warn!("No per-user cache directory available; core falls back to temp dir"),
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Attune")
            .with_app_id("org.mizproject.attune")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([780.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Attune",
        options,
        Box::new(|cc| Ok(Box::new(app::AttuneApp::new(cc)))),
    )
}
