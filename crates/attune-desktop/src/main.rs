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
pub mod system_fonts;
mod theme;

#[cfg(target_os = "linux")]
mod linux_xdo_shim {
    //! Cross-distro `libxdo` compatibility resolver.
    //!
    //! `muda` (pulled in by `dioxus-desktop`) links against `libxdo` for synthetic menu
    //! keystrokes, but Ubuntu/Debian ship `libxdo.so.3` while Arch/Fedora ship `libxdo.so.4`,
    //! and minimal desktop installs may not have `xdotool` installed at all.
    //!
    //! By providing the 3 C-ABI entrypoints (`xdo_new`, `xdo_free`, `xdo_send_keysequence_window`)
    //! directly and resolving any installed `libxdo.so.{4,3}` via `dlopen` at runtime on demand,
    //! combined with `-Wl,--as-needed` in `build.rs`, the `attune-desktop` ELF binary has zero
    //! hard `DT_NEEDED` binding to a specific `libxdo.so.<N>` SONAME.
    use std::ffi::{c_char, c_int, c_ulong, c_void};
    use std::sync::OnceLock;

    struct XdoApi {
        xdo_new: unsafe extern "C" fn(*const c_char) -> *mut c_void,
        xdo_free: unsafe extern "C" fn(*mut c_void),
        xdo_send_keysequence_window:
            unsafe extern "C" fn(*const c_void, c_ulong, *const c_char, u32) -> c_int,
    }

    static XDO_API: OnceLock<Option<XdoApi>> = OnceLock::new();

    fn load_xdo_api() -> &'static Option<XdoApi> {
        XDO_API.get_or_init(|| unsafe {
            let candidates: [&[u8]; 4] = [
                b"libxdo.so.4\0",
                b"libxdo.so.3\0",
                b"libxdo.so\0",
                b"libxdo.so.2\0",
            ];
            for cand in candidates {
                let handle = libc::dlopen(cand.as_ptr() as *const c_char, libc::RTLD_LAZY | libc::RTLD_LOCAL);
                if !handle.is_null() {
                    let s_new = libc::dlsym(handle, b"xdo_new\0".as_ptr() as *const c_char);
                    let s_free = libc::dlsym(handle, b"xdo_free\0".as_ptr() as *const c_char);
                    let s_send = libc::dlsym(
                        handle,
                        b"xdo_send_keysequence_window\0".as_ptr() as *const c_char,
                    );
                    if !s_new.is_null() && !s_free.is_null() && !s_send.is_null() {
                        return Some(XdoApi {
                            xdo_new: std::mem::transmute(s_new),
                            xdo_free: std::mem::transmute(s_free),
                            xdo_send_keysequence_window: std::mem::transmute(s_send),
                        });
                    }
                    libc::dlclose(handle);
                }
            }
            None
        })
    }

    #[no_mangle]
    pub unsafe extern "C" fn xdo_new(display: *const c_char) -> *mut c_void {
        if let Some(ref api) = load_xdo_api() {
            (api.xdo_new)(display)
        } else {
            std::ptr::null_mut()
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn xdo_free(xdo: *mut c_void) {
        if !xdo.is_null() {
            if let Some(ref api) = load_xdo_api() {
                (api.xdo_free)(xdo);
            }
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn xdo_send_keysequence_window(
        xdo: *const c_void,
        window: c_ulong,
        keysequence: *const c_char,
        delay: u32,
    ) -> c_int {
        if !xdo.is_null() {
            if let Some(ref api) = load_xdo_api() {
                return (api.xdo_send_keysequence_window)(xdo, window, keysequence, delay);
            }
        }
        0
    }
}

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
            "<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\n\
            <link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin>\n\
            <link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&family=Plus+Jakarta+Sans:wght@400;500;600;700;800&family=Roboto:wght@400;500;700&family=Sora:wght@400;600;700;800&display=swap\">\n\
            <link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/icon?family=Material+Icons\">\n\
            <style>\n{icons_css}\n{css}\n</style>\n{}\n{}\n{}",
            crate::theme::EVENT_COORDINATE_SANITIZER_SCRIPT,
            crate::theme::MARQUEE_SCRIPT,
            crate::theme::LYRICS_SYNC_SCRIPT
        ));

    dioxus::LaunchBuilder::desktop()
        .with_cfg(cfg)
        .launch(app::App);
}

#[cfg(all(test, target_os = "linux"))]
mod main_tests {
    use super::linux_xdo_shim;

    #[test]
    fn test_linux_xdo_shim_null_safety() {
        unsafe {
            // Passing null to xdo_free and xdo_send_keysequence_window must be a safe no-op
            linux_xdo_shim::xdo_free(std::ptr::null_mut());
            let rc = linux_xdo_shim::xdo_send_keysequence_window(
                std::ptr::null(),
                0,
                b"Return\0".as_ptr() as *const std::ffi::c_char,
                0,
            );
            assert_eq!(rc, 0);
        }
    }
}
