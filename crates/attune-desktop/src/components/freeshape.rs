//! Material You Expressive Freeshapes generator and morphing clipper for egui.
//!
//! Evaluates polar harmonic shapes:
//!   r(theta) = R * [1.0 + sum(a_k * cos(k * theta + phi_k))]
//! with dynamic morphing:
//!   r(theta, t) = (1 - t) * r_from(theta) + t * r_to(theta)
//! and a ring-buffer recency penalty preventing repeated shapes on track change.

use std::collections::VecDeque;
use std::f32::consts::PI;
use eframe::egui::{Color32, Pos2, Rect, Stroke, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreeshapeArchetype {
    FourLeafFlower,
    EightPointStar,
    SmoothPebble,
    SquircleSuperellipse,
    TwelveFacetSunburst,
}

impl FreeshapeArchetype {
    pub const ALL: [FreeshapeArchetype; 5] = [
        FreeshapeArchetype::FourLeafFlower,
        FreeshapeArchetype::EightPointStar,
        FreeshapeArchetype::SmoothPebble,
        FreeshapeArchetype::SquircleSuperellipse,
        FreeshapeArchetype::TwelveFacetSunburst,
    ];

    pub fn radius_at(self, theta: f32) -> f32 {
        match self {
            FreeshapeArchetype::FourLeafFlower => {
                // k = 4 harmonic clover
                1.0 + 0.24 * (4.0 * theta).cos()
            }
            FreeshapeArchetype::EightPointStar => {
                // k = 8 expressive star
                1.0 + 0.18 * (8.0 * theta).cos()
            }
            FreeshapeArchetype::SmoothPebble => {
                // Asymmetric gentle wave
                1.0 + 0.16 * (3.0 * theta).cos() + 0.08 * (5.0 * theta + 0.8).sin()
            }
            FreeshapeArchetype::SquircleSuperellipse => {
                // Superellipse (|cos|^4 + |sin|^4)^(-1/4)
                let c = theta.cos().abs().powi(4);
                let s = theta.sin().abs().powi(4);
                let r = (c + s).powf(-0.25);
                0.85 * r
            }
            FreeshapeArchetype::TwelveFacetSunburst => {
                // k = 12 scalloped badge
                1.0 + 0.10 * (12.0 * theta).cos()
            }
        }
    }
}

/// Tracks recency of shapes to prevent repeating the same shape consecutively.
#[derive(Debug, Clone)]
pub struct FreeshapeRecencyTracker {
    recent_shapes: VecDeque<FreeshapeArchetype>,
    max_history: usize,
    pub current_shape: FreeshapeArchetype,
    pub target_shape: FreeshapeArchetype,
    pub morph_progress: f32, // 0.0 to 1.0
}

impl Default for FreeshapeRecencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl FreeshapeRecencyTracker {
    pub fn new() -> Self {
        Self {
            recent_shapes: VecDeque::new(),
            max_history: 3,
            current_shape: FreeshapeArchetype::FourLeafFlower,
            target_shape: FreeshapeArchetype::FourLeafFlower,
            morph_progress: 1.0,
        }
    }

    /// Selects the next shape based on a seed (e.g. track title hash), penalizing recent shapes.
    pub fn trigger_next_shape(&mut self, seed: u64) {
        let available: Vec<FreeshapeArchetype> = FreeshapeArchetype::ALL
            .iter()
            .copied()
            .filter(|s| !self.recent_shapes.contains(s))
            .collect();

        let pool = if available.is_empty() {
            &FreeshapeArchetype::ALL[..]
        } else {
            &available[..]
        };

        let next = pool[(seed as usize) % pool.len()];
        self.current_shape = self.target_shape;
        self.target_shape = next;
        self.morph_progress = 0.0;

        self.recent_shapes.push_back(next);
        while self.recent_shapes.len() > self.max_history {
            self.recent_shapes.pop_front();
        }
    }

    pub fn step_morph(&mut self, dt: f32) {
        if self.morph_progress < 1.0 {
            self.morph_progress = (self.morph_progress + dt * 2.5).min(1.0);
        }
    }

    /// Generates polygon points for the morphing shape.
    pub fn generate_points(&self, center: Pos2, base_radius: f32, num_steps: usize) -> Vec<Pos2> {
        let mut points = Vec::with_capacity(num_steps);
        let t = self.morph_progress;

        for i in 0..num_steps {
            let theta = (i as f32 / num_steps as f32) * 2.0 * PI;
            let r_cur = self.current_shape.radius_at(theta);
            let r_tar = self.target_shape.radius_at(theta);
            let r = base_radius * ((1.0 - t) * r_cur + t * r_tar);

            let x = center.x + r * theta.cos();
            let y = center.y + r * theta.sin();
            points.push(Pos2::new(x, y));
        }

        points
    }
}

/// Paints an expressive freeshape container behind an album artwork thumbnail or cutout.
pub fn paint_freeshape_backdrop(
    ui: &Ui,
    rect: Rect,
    tracker: &FreeshapeRecencyTracker,
    fill_color: Color32,
    border_color: Color32,
) {
    let center = rect.center();
    let base_radius = rect.width().min(rect.height()) * 0.45;
    let points = tracker.generate_points(center, base_radius, 96);

    if points.len() >= 3 {
        ui.painter().add(eframe::egui::epaint::PathShape::convex_polygon(
            points.clone(),
            fill_color,
            Stroke::new(2.0, border_color),
        ));
    }
}
