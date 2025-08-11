//! Visualization module
//!
//! This module handles rendering and visualization systems including
//! arrows, axes, and configuration for visual elements.

use bevy::prelude::*;

pub mod arrows;
pub mod axes;
pub mod config;

pub use config::ArrowConfig;
pub use arrows::draw_city_to_satellite_arrows;
pub use axes::{draw_axes, ShowAxes};

/// Plugin for visualization systems
pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArrowConfig>()
            .add_systems(
                Update,
                (
                    draw_axes,
                    draw_city_to_satellite_arrows,
                ),
            );
    }
}