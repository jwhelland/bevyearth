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
pub mod launches;
pub mod lighting;
pub mod moon;
pub mod sky_material;

pub use arrows::draw_city_to_satellite_arrows;
#[allow(unused_imports)]
pub use axes::{ShowAxes, draw_axes};
#[allow(unused_imports)]
pub use cities::{CitiesEcef, CitiesPlugin};
pub use config::ArrowConfig;
#[allow(unused_imports)]
pub use earth::EarthPlugin;
#[allow(unused_imports)]
pub use ground_track::{GroundTrackConfig, GroundTrackPlugin};
#[allow(unused_imports)]
pub use ground_track_gizmo::{GroundTrackGizmoConfig, GroundTrackGizmoPlugin};
#[allow(unused_imports)]
pub use heatmap::{HeatmapConfig, HeatmapPlugin, RangeMode};
#[allow(unused_imports)]
pub use launches::LaunchesPlugin;
#[allow(unused_imports)]
pub use lighting::SunLight;
#[allow(unused_imports)]
pub use moon::MoonPlugin;
pub use sky_material::SkyMaterialPlugin;

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
                    lighting::update_sun_light_direction,
                ),
            )
            .add_plugins(SkyMaterialPlugin);
    }
}
