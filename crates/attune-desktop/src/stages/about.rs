//! About, Architecture & Distribution Stage for Attune egui.
//!
//! Complies with ADR-033, ADR-058 & Mandate 4:
//! - Credits project author (Mizumo-prjkt)
//! - Discloses AI pair programming with Google DeepMind Antigravity AI assistant
//! - Provides Obtainium Android auto-update card with deep link & web redirect

use eframe::egui::{self, Color32, Ui};
use crate::components::glass::liquid_glass_card;
use crate::config;

const OBTAINIUM_REDIRECT_URL: &str = "https://apps.obtainium.imranr.dev/redirect?r=obtainium://app/%7B%22id%22%3A%22org.mizproject.attune%22%2C%22url%22%3A%22https%3A%2F%2Fgithub.com%2FMizumo-prjkt%2Fattune%22%2C%22author%22%3A%22Mizumo-prjkt%22%2C%22name%22%3A%22Attune%22%2C%22additionalSettings%22%3A%22%7B%5C%22about%5C%22%3A%5C%22Open-source%20DJ-grade%20audio-profile-driven%20transition%20and%20remixing%20music%20player.%5C%22%7D%22%7D";

pub struct AboutStage;

impl AboutStage {
    pub fn show(ui: &mut Ui) {
        ui.heading(egui::RichText::new("About Attune").strong().color(crate::theme::ACCENT));
        ui.label(egui::RichText::new(format!("Version: v{}", config::app_version())).monospace().weak());

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);

            // Project Overview Card
            liquid_glass_card(ui, 16.0, crate::theme::ACCENT, |ui| {
                ui.heading(egui::RichText::new("Attune Music Player").size(18.0).strong());
                ui.label(egui::RichText::new("DJ-grade audio-profile-driven transition & remixing engine").weak());
                ui.add_space(8.0);
                ui.label("Attune analyzes sustained vocal pitch, 12-bin CQT local harmonic chroma, beat & phrase grids, and band-split spectral energy to execute seamless, musically intelligent transitions in real-time.");
            });

            ui.add_space(14.0);

            // Credits & Architecture Card
            liquid_glass_card(ui, 16.0, Color32::from_rgb(0xA8, 0x55, 0xF7), |ui| {
                ui.strong("Credits & Core Engineering");
                ui.add_space(4.0);
                ui.label("• Author & Lead Architect: Mizumo-prjkt <bluepearl394@gmail.com>");
                ui.label("• AI Pair Programming Collaboration: Google DeepMind Antigravity AI Assistant");
                ui.label("• Protocol: Attune Knowledge Ledger System (ledger/00_MASTER_LEDGER.md)");
                ui.label("• Audio Primitives: Symphonia, CPAL, Hound, RealFFT, Blake3, SQLite (rusqlite)");
                ui.label("• Native Desktop GUI: Rust eframe / egui 0.33 with glow OpenGL backend");
            });

            ui.add_space(14.0);

            // Obtainium Android Auto-Update Card
            liquid_glass_card(ui, 16.0, Color32::from_rgb(0x2D, 0xD4, 0xBF), |ui| {
                ui.horizontal(|ui| {
                    ui.strong("📱 Obtainium Android Updates");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.hyperlink_to("🔗 Open Redirect Page", OBTAINIUM_REDIRECT_URL);
                    });
                });
                ui.add_space(6.0);
                ui.label("Track, install, and automatically receive new Attune updates directly from GitHub Releases using Obtainium on Android devices.");
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("📋 Copy Obtainium Deep Link").clicked() {
                        ui.ctx().copy_text(OBTAINIUM_REDIRECT_URL.to_string());
                    }
                    ui.label(egui::RichText::new("(Redirects to obtainium://app/...)").small().weak());
                });
            });

            ui.add_space(14.0);

            // License & Disclosure Card
            liquid_glass_card(ui, 16.0, Color32::from_white_alpha(40), |ui| {
                ui.strong("Open Source License");
                ui.add_space(4.0);
                ui.label("Attune is licensed under the MIT License / Apache-2.0 Dual License.");
                ui.label("Copyright (c) 2026 Mizumo-prjkt and Attune Contributors.");
            });

            ui.add_space(30.0);
        });
    }
}
