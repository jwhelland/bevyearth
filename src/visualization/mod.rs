//! Visualization module
//!
//! This module handles rendering and visualization systems including
//! arrows, axes, and configuration for visual elements.

use bevy::prelude::*;

pub mod arrows;
pub mod axes;
pub mod cities;
pub mod colormaps;
pub mod config;
pub mod earth;
pub mod ground_track;
pub mod ground_track_gizmo;
pub mod heatmap;

pub use arrows::draw_city_to_satellite_arrows;
pub use axes::{ShowAxes, draw_axes};
pub use cities::{CitiesEcef, CitiesPlugin};
pub use config::ArrowConfig;
pub use earth::EarthPlugin;
pub use ground_track::{GroundTrackConfig, GroundTrackPlugin};
pub use ground_track_gizmo::{GroundTrackGizmoConfig, GroundTrackGizmoPlugin};
pub use heatmap::{HeatmapConfig, HeatmapPlugin, RangeMode};

/// Plugin for visualization systems
pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArrowConfig>()
            .add_systems(Update, (draw_axes, draw_city_to_satellite_arrows));
    }
}
