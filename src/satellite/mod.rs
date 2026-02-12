//! Satellite management module
//!
//! This module handles satellite components, resources, and systems for tracking
//! and managing satellite entities in the Bevy ECS.

use bevy::prelude::*;

pub mod components;
pub mod resources;
pub mod systems;

pub use components::{Satellite, SatelliteColor};
pub use resources::{
    ColorHueCounter, GroupMaterialCache, NoradIndex, OrbitTrailConfig, SatelliteRenderConfig,
    SelectedSatellite,
};
pub use systems::{
    draw_orbit_trails_system, init_satellite_render_assets, materialize_satellite_entities_system,
    move_camera_to_satellite, propagate_satellites_system, satellite_click_system,
    track_satellite_continuously, update_group_colors_system, update_orbit_trails_system,
    update_satellite_rendering_system,
};

/// Plugin for satellite management and propagation
pub struct SatellitePlugin;

impl Plugin for SatellitePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedSatellite>()
            .init_resource::<GroupMaterialCache>()
            .init_resource::<NoradIndex>()
            .init_resource::<ColorHueCounter>()
            .insert_resource(crate::ui::groups::initialize_group_registry())
            // OrbitTrailConfig and SatelliteRenderConfig are now in UiConfigBundle
            .add_systems(Startup, init_satellite_render_assets)
            .add_systems(
                Update,
                (
                    materialize_satellite_entities_system,
                    propagate_satellites_system.after(materialize_satellite_entities_system),
                    update_orbit_trails_system.after(propagate_satellites_system),
                    draw_orbit_trails_system.after(update_orbit_trails_system),
                    update_satellite_rendering_system,
                    update_group_colors_system,
                    move_camera_to_satellite,
                    track_satellite_continuously.after(propagate_satellites_system),
                    satellite_click_system,
                ),
            );
    }
}
