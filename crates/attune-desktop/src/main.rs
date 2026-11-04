//! Attune Desktop — native Rust Dioxus frontend for Windows and Linux.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod app_icon;
mod components;
mod config;
mod library;
mod player;
mod smart_shuffle;
mod stages;
mod theme;

use dioxus::desktop::{Config, WindowBuilder};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    match config::ensure_cache_dir_env() {
        Some(dir) => log::info!("Attune cache directory: {}", dir.display()),
        None => log::warn!("No per-user cache directory available; core falls back to temp dir"),
    }

    let icon = app_icon::load_app_icon();
    let mut window = WindowBuilder::new()
        .with_title("Attune")
        .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(1280.0, 820.0))
        .with_min_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(880.0, 560.0));

    if let Some(ic) = icon {
        window = window.with_window_icon(Some(ic));
    }

    let css = include_str!("style.css");
    let cfg = Config::new()
        .with_window(window)
        .with_menu(None)
        .with_custom_head(format!("<style>{css}</style>"));

    dioxus::LaunchBuilder::desktop()
        .with_cfg(cfg)
        .launch(app::App);
}
