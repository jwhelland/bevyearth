//! Visualization configuration

use bevy::prelude::*;

/// Arrow rendering configuration resource
#[derive(Resource)]
pub struct ArrowConfig {
    pub enabled: bool,
    pub color: Color,
    pub max_visible: usize,
    pub lift_m: f32,
    #[allow(dead_code)]
    pub head_len_pct: f32,
    pub head_min_m: f32,
    pub head_max_m: f32,
    #[allow(dead_code)]
    pub head_radius_pct: f32,
    pub shaft_len_pct: f32,
    pub shaft_min_m: f32,
    pub shaft_max_m: f32,
    pub gradient_enabled: bool,
    pub gradient_near_km: f32,
    pub gradient_far_km: f32,
    pub gradient_near_color: Color,
    pub gradient_far_color: Color,
    pub gradient_log_scale: bool,
}

impl Default for ArrowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            color: Color::srgb(0.1, 0.9, 0.3),
            max_visible: 200,
            lift_m: 10000.0,
            head_len_pct: 0.02,
            head_min_m: 10_000.0,
            head_max_m: 100_000.0,
            head_radius_pct: 0.4,
            shaft_len_pct: 0.05,
            shaft_min_m: 1_000.0,
            shaft_max_m: 400_000.0,
            gradient_enabled: false,
            gradient_near_km: 1000.0,
            gradient_far_km: 60000.0,
            gradient_near_color: Color::srgb(1.0, 0.0, 0.0),
            gradient_far_color: Color::srgb(0.0, 0.0, 1.0),
            gradient_log_scale: false,
        }
    }
}
