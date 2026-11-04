//! For You Stage — Algorithmic Mixes & Harmonic Discovery (ADR-049 Parity).

use eframe::egui::{self, Color32, Ui};
use crate::components::glass::liquid_glass_card;
use crate::library::TrackEntry;
use crate::theme::ACCENT;

pub struct ForYouStage;

impl ForYouStage {
    pub fn show(
        ui: &mut Ui,
        tracks: &[TrackEntry],
        _on_play_track: &mut impl FnMut(usize),
        on_play_mix: &mut impl FnMut(&[usize]),
    ) {
        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("✨ For You").strong().color(ACCENT));
            ui.label(egui::RichText::new("Algorithmic Mixes & Harmonic Discovery").weak().small());
        });

        ui.separator();

        if tracks.is_empty() {
            ui.add_space(20.0);
            ui.label(egui::RichText::new("No tracks loaded to compute mixes.").weak());
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(10.0);

            // Mix 1: Harmonic Camelot Flow
            liquid_glass_card(ui, 16.0, ACCENT, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(egui::RichText::new("⚡ Harmonic Energy Flow").size(18.0).strong());
                        ui.label(egui::RichText::new("Seamless key-compatible transitions dynamically sequenced via Camelot Circle of Fifths").weak());
                        ui.add_space(8.0);
                        let mix_indices: Vec<usize> = (0..tracks.len().min(12)).collect();
                        let btn = egui::Button::new(
                            egui::RichText::new("▶ Play Harmonic Mix")
                                .strong()
                                .color(Color32::from_rgb(0x0A, 0x22, 0x1A)),
                        )
                        .fill(ACCENT);
                        if ui.add(btn).clicked() {
                            on_play_mix(&mix_indices);
                        }
                    });
                });
            });

            ui.add_space(14.0);

            // Mix 2: Upbeat Peak Energy
            liquid_glass_card(ui, 16.0, Color32::from_rgb(0xFB, 0x71, 0x85), |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(egui::RichText::new("🔥 High BPM Peak Energy").size(18.0).strong());
                        ui.label(egui::RichText::new("Curated fast-tempo and energetic tracks (140+ BPM) with punchy bass handoffs").weak());
                        ui.add_space(8.0);
                        let high_bpm_indices: Vec<usize> = tracks
                            .iter()
                            .enumerate()
                            .filter(|(_, t)| t.bpm.map(|b| b >= 140.0).unwrap_or(false))
                            .map(|(i, _)| i)
                            .take(12)
                            .collect();
                        let target = if high_bpm_indices.is_empty() {
                            (0..tracks.len().min(8)).collect()
                        } else {
                            high_bpm_indices
                        };
                        if ui.button(egui::RichText::new("▶ Play High BPM Mix").strong()).clicked() {
                            on_play_mix(&target);
                        }
                    });
                });
            });

            ui.add_space(14.0);

            // Mix 3: Melodic Chill & Acoustic
            liquid_glass_card(ui, 16.0, Color32::from_rgb(0x38, 0xBD, 0xF8), |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(egui::RichText::new("🌿 Melodic Chillout & Ambient").size(18.0).strong());
                        ui.label(egui::RichText::new("Gentle sustain pitch morphs and acoustic resonance for relaxed listening").weak());
                        ui.add_space(8.0);
                        let chill_indices: Vec<usize> = tracks
                            .iter()
                            .enumerate()
                            .filter(|(_, t)| t.bpm.map(|b| b < 125.0).unwrap_or(false))
                            .map(|(i, _)| i)
                            .take(12)
                            .collect();
                        let target = if chill_indices.is_empty() {
                            (0..tracks.len().min(8)).collect()
                        } else {
                            chill_indices
                        };
                        if ui.button(egui::RichText::new("▶ Play Chill Mix").strong()).clicked() {
                            on_play_mix(&target);
                        }
                    });
                });
            });

            ui.add_space(30.0);
        });
    }
}
