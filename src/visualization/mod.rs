//! Visualization module
//!
//! This module handles rendering and visualization systems including
//! arrows, axes, and configuration for visual elements.

use bevy::prelude::*;

pub mod arrows;
pub mod axes;
pub mod config;

pub use arrows::draw_city_to_satellite_arrows;
pub use axes::{ShowAxes, draw_axes};
pub use config::ArrowConfig;

/// Plugin for visualization systems
pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArrowConfig>()
            .add_systems(Update, (draw_axes, draw_city_to_satellite_arrows));
    }
}
