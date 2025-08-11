//! Satellite management module
//!
//! This module handles satellite components, resources, and systems for tracking
//! and managing satellite entities in the Bevy ECS.

use bevy::prelude::*;

pub mod components;
pub mod resources;
pub mod systems;

pub use components::{Satellite, SatelliteColor};
pub use resources::{SatelliteStore, SatEntry, SatEcef, OrbitTrailConfig};
pub use systems::{
    propagate_satellites_system, 
    update_satellite_ecef, 
    spawn_missing_satellite_entities_system,
    update_orbit_trails_system,
    draw_orbit_trails_system
};

/// Plugin for satellite management and propagation
pub struct SatellitePlugin;

impl Plugin for SatellitePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SatEcef>()
            .init_resource::<SatelliteStore>()
            .init_resource::<OrbitTrailConfig>()
            .add_systems(
                Update,
                (
                    spawn_missing_satellite_entities_system,
                    propagate_satellites_system.after(spawn_missing_satellite_entities_system),
                    update_satellite_ecef.after(propagate_satellites_system),
                    update_orbit_trails_system.after(propagate_satellites_system),
                    draw_orbit_trails_system.after(update_orbit_trails_system),
                ),
            );
    }
}