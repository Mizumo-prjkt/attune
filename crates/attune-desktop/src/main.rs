//! Attune Desktop — native Rust Dioxus frontend for Windows and Linux.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod app_icon;
pub mod artist_profile;
mod components;
mod config;
mod dynamic_components;
pub mod i18n;
mod library;
mod player;
mod process_manager;
mod smart_shuffle;
mod stages;
mod theme;

use dioxus::desktop::{Config, WindowBuilder};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Mandate: Strictly verify that all external dynamic shared libraries (.so / .dll)
    // are present and loadable. If any are missing, pop an explicit error dialog and exit.
    if let Err(missing_components) = dynamic_components::preflight_check_and_load() {
        let mut msg = String::from("Attune Desktop failed to initialize required external components:\n\n");
        for err in &missing_components {
            msg.push_str(&format!("• Missing: {}\n", err.library_filename));
            msg.push_str(&format!("  Component: {}\n", err.component_name));
            msg.push_str("  Searched paths:\n");
            for p in &err.searched_paths {
                msg.push_str(&format!("    - {}\n", p.display()));
            }
            msg.push_str(&format!("  Diagnostic: {}\n\n", err.error_details));
        }
        msg.push_str("Please ensure that the required shared libraries (.so / .dll) are located in the application directory, ./lib/, or system library path.");

        eprintln!("================================================================================");
        eprintln!("FATAL ERROR: MISSING ATTUNE DYNAMIC SHARED LIBRARIES");
        eprintln!("================================================================================");
        eprintln!("{msg}");
        log::error!("{msg}");

        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Attune — Missing Dynamic Shared Libraries")
            .set_description(&msg)
            .show();

        std::process::exit(1);
    }

    // Configure watchguard for desktop hardware telemetry evaluation mode
    let _ = dynamic_components::watchguard::set_desktop_mode(true);

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
    let icons_css = include_str!("material_icons.css");
    let cfg = Config::new()
        .with_window(window)
        .with_menu(None)
        .with_custom_head(format!(
            "<link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/icon?family=Material+Icons\">\n<style>\n{icons_css}\n{css}\n</style>"
        ));

    dioxus::LaunchBuilder::desktop()
        .with_cfg(cfg)
        .launch(app::App);
}
