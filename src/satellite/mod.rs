//! Satellite management module
//!
//! This module handles satellite components, resources, and systems for tracking
//! and managing satellite entities in the Bevy ECS.

use bevy::prelude::*;

pub mod components;
pub mod resources;
pub mod systems;

pub use components::{Satellite, SatelliteColor};
pub use resources::{OrbitTrailConfig, SatEcef, SatEntry, SatelliteStore, SelectedSatellite};
pub use systems::{
    draw_orbit_trails_system, move_camera_to_satellite, propagate_satellites_system,
    satellite_click_system, spawn_missing_satellite_entities_system, track_satellite_continuously,
    update_orbit_trails_system, update_satellite_ecef,
};

/// Plugin for satellite management and propagation
pub struct SatellitePlugin;

impl Plugin for SatellitePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SatEcef>()
            .init_resource::<SatelliteStore>()
            .init_resource::<OrbitTrailConfig>()
            .init_resource::<SelectedSatellite>()
            .add_systems(
                Update,
                (
                    spawn_missing_satellite_entities_system,
                    propagate_satellites_system.after(spawn_missing_satellite_entities_system),
                    update_satellite_ecef.after(propagate_satellites_system),
                    update_orbit_trails_system.after(propagate_satellites_system),
                    draw_orbit_trails_system.after(update_orbit_trails_system),
                    move_camera_to_satellite,
                    track_satellite_continuously.after(propagate_satellites_system),
                    satellite_click_system,
                ),
            );
    }
}
