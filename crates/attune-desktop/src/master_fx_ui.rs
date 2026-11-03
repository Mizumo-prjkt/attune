//! Interactive Master FX DSP Rack & 10-Band Graphic Equalizer for Attune egui.
//!
//! Provides real-time parameter tweaking for the in-process mastering chain:
//! - 10-band cascaded IIR graphic EQ (31Hz to 16kHz)
//! - Attune Pure Bass+ sub-harmonic synthesizer
//! - Attune Clarity & Vocal Air exciter
//! - Mid-Side Stereo Expander
//! - Brickwall Limiter & Headroom Pre-Amp

use attune_core::{MasterFxParams, EQ_BANDS_COUNT, EQ_CENTER_FREQS};
use eframe::egui::{self, Color32, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqPreset {
    Flat,
    BassBoost,
    VocalClarity,
    Electronic,
    Rock,
    Acoustic,
}

impl EqPreset {
    pub const ALL: [EqPreset; 6] = [
        EqPreset::Flat,
        EqPreset::BassBoost,
        EqPreset::VocalClarity,
        EqPreset::Electronic,
        EqPreset::Rock,
        EqPreset::Acoustic,
    ];

    pub fn name(self) -> &'static str {
        match self {
            EqPreset::Flat => "Flat",
            EqPreset::BassBoost => "Bass Boost",
            EqPreset::VocalClarity => "Vocal Clarity",
            EqPreset::Electronic => "Electronic / Dance",
            EqPreset::Rock => "Rock / Punch",
            EqPreset::Acoustic => "Acoustic / Warm",
        }
    }

    pub fn apply(self, params: &mut MasterFxParams) {
        match self {
            EqPreset::Flat => {
                params.band_gains_db = [0.0; EQ_BANDS_COUNT];
                params.bass_enabled = false;
                params.clarity_enabled = false;
                params.stereo_width = 1.0;
                params.preamp_db = -3.0;
            }
            EqPreset::BassBoost => {
                params.band_gains_db = [5.5, 4.5, 3.0, 1.5, 0.0, 0.0, 0.0, 1.0, 1.5, 2.0];
                params.bass_enabled = true;
                params.bass_gain_db = 6.0;
                params.bass_freq_hz = 75.0;
                params.clarity_enabled = false;
                params.stereo_width = 1.1;
                params.preamp_db = -4.5;
            }
            EqPreset::VocalClarity => {
                params.band_gains_db = [-1.0, -0.5, 0.0, 1.0, 2.5, 3.5, 4.0, 3.0, 2.5, 2.0];
                params.bass_enabled = false;
                params.clarity_enabled = true;
                params.clarity_wet = 0.20;
                params.stereo_width = 1.15;
                params.preamp_db = -3.5;
            }
            EqPreset::Electronic => {
                params.band_gains_db = [6.0, 5.0, 2.5, -0.5, -1.0, 0.5, 2.0, 3.5, 4.5, 5.0];
                params.bass_enabled = true;
                params.bass_gain_db = 5.0;
                params.bass_freq_hz = 80.0;
                params.clarity_enabled = true;
                params.clarity_wet = 0.15;
                params.stereo_width = 1.25;
                params.preamp_db = -4.0;
            }
            EqPreset::Rock => {
                params.band_gains_db = [4.0, 3.0, 1.5, 2.0, 2.5, 1.5, 2.0, 3.0, 2.5, 2.0];
                params.bass_enabled = false;
                params.clarity_enabled = true;
                params.clarity_wet = 0.12;
                params.stereo_width = 1.10;
                params.preamp_db = -3.5;
            }
            EqPreset::Acoustic => {
                params.band_gains_db = [2.0, 2.5, 1.5, 1.0, 1.5, 2.0, 1.5, 1.0, 1.5, 2.0];
                params.bass_enabled = false;
                params.clarity_enabled = false;
                params.stereo_width = 1.05;
                params.preamp_db = -2.5;
            }
        }
    }
}

pub struct MasterFxUi {
    pub is_open: bool,
    pub params: MasterFxParams,
    pub active_preset: Option<EqPreset>,
}

impl Default for MasterFxUi {
    fn default() -> Self {
        Self {
            is_open: false,
            params: MasterFxParams::default(),
            active_preset: Some(EqPreset::Flat),
        }
    }
}

impl MasterFxUi {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ctx: &egui::Context, on_change: impl FnOnce(&MasterFxParams)) {
        if !self.is_open {
            return;
        }

        let mut changed = false;
        let mut is_open = self.is_open;

        egui::Window::new("Attune Studio Master FX & 10-Band EQ")
            .open(&mut is_open)
            .resizable(true)
            .default_width(720.0)
            .min_width(600.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut self.params.enabled, "Master FX Enabled").changed() {
                        changed = true;
                    }
                    ui.separator();
                    ui.label("Presets:");
                    for preset in EqPreset::ALL {
                        let selected = self.active_preset == Some(preset);
                        if ui.selectable_label(selected, preset.name()).clicked() {
                            preset.apply(&mut self.params);
                            self.active_preset = Some(preset);
                            changed = true;
                        }
                    }
                });

                ui.separator();

                // 10-Band Graphic EQ Visualizer & Sliders
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong("10-Band Graphic Equalizer");
                        ui.label(egui::RichText::new("(-12 dB to +12 dB)").weak().small());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Reset EQ Bands").clicked() {
                                self.params.band_gains_db = [0.0; EQ_BANDS_COUNT];
                                self.active_preset = Some(EqPreset::Flat);
                                changed = true;
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Curve preview
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 60.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 6.0, ui.visuals().extreme_bg_color);
                    let zero_y = rect.center().y;
                    ui.painter().line_segment(
                        [egui::pos2(rect.left(), zero_y), egui::pos2(rect.right(), zero_y)],
                        Stroke::new(1.0, Color32::from_white_alpha(30)),
                    );

                    let mut points = Vec::new();
                    for (i, &gain) in self.params.band_gains_db.iter().enumerate() {
                        let x = rect.left() + (rect.width() * (i as f32 / (EQ_BANDS_COUNT - 1) as f32));
                        let norm = (-gain / 12.0).clamp(-1.0, 1.0);
                        let y = zero_y + (norm * (rect.height() * 0.40));
                        points.push(egui::pos2(x, y));
                    }
                    if points.len() >= 2 {
                        ui.painter().add(egui::epaint::PathShape::line(
                            points,
                            Stroke::new(2.5, crate::theme::ACCENT),
                        ));
                    }

                    ui.add_space(8.0);

                    // Vertical Sliders
                    ui.columns(EQ_BANDS_COUNT, |cols| {
                        for i in 0..EQ_BANDS_COUNT {
                            cols[i].vertical_centered(|ui| {
                                let freq_label = if EQ_CENTER_FREQS[i] >= 1000.0 {
                                    format!("{:.0}k", EQ_CENTER_FREQS[i] / 1000.0)
                                } else {
                                    format!("{:.0}", EQ_CENTER_FREQS[i])
                                };
                                ui.label(egui::RichText::new(freq_label).small().monospace());
                                let slider = ui.add(
                                    egui::Slider::new(&mut self.params.band_gains_db[i], -12.0..=12.0)
                                        .vertical()
                                        .show_value(false),
                                );
                                if slider.changed() {
                                    self.active_preset = None;
                                    changed = true;
                                }
                                let val_text = format!("{:+.1}", self.params.band_gains_db[i]);
                                ui.label(egui::RichText::new(val_text).small().weak());
                            });
                        }
                    });
                });

                ui.add_space(10.0);

                // Pure Bass+ & Clarity & Stereo Controls
                ui.columns(3, |cols| {
                    cols[0].group(|ui| {
                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut self.params.bass_enabled, "Pure Bass+").changed() {
                                changed = true;
                            }
                        });
                        ui.label(egui::RichText::new("Sub-harmonic synthesizer").weak().small());
                        ui.add_enabled_ui(self.params.bass_enabled, |ui| {
                            ui.label("Boost (dB):");
                            if ui.add(egui::Slider::new(&mut self.params.bass_gain_db, 0.0..=12.0).suffix(" dB")).changed() {
                                changed = true;
                            }
                            ui.label("Cutoff Frequency:");
                            if ui.add(egui::Slider::new(&mut self.params.bass_freq_hz, 40.0..=140.0).suffix(" Hz")).changed() {
                                changed = true;
                            }
                        });
                    });

                    cols[1].group(|ui| {
                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut self.params.clarity_enabled, "Clarity & Vocal Air").changed() {
                                changed = true;
                            }
                        });
                        ui.label(egui::RichText::new("High-shelf harmonic exciter").weak().small());
                        ui.add_enabled_ui(self.params.clarity_enabled, |ui| {
                            ui.label("Air Wet Level:");
                            if ui.add(egui::Slider::new(&mut self.params.clarity_wet, 0.0..=0.35).fixed_decimals(2)).changed() {
                                changed = true;
                            }
                        });
                    });

                    cols[2].group(|ui| {
                        ui.strong("Spatial & Headroom");
                        ui.label(egui::RichText::new("Mid-side widen & pre-amp").weak().small());
                        ui.label("Stereo Width:");
                        if ui.add(egui::Slider::new(&mut self.params.stereo_width, 0.5..=2.0).suffix("x").fixed_decimals(2)).changed() {
                            changed = true;
                        }
                        ui.label("Pre-Amp Pad:");
                        if ui.add(egui::Slider::new(&mut self.params.preamp_db, -12.0..=4.0).suffix(" dB").fixed_decimals(1)).changed() {
                            changed = true;
                        }
                    });
                });
            });

        self.is_open = is_open;
        if changed {
            on_change(&self.params);
        }
    }
}
