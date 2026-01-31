//! Satellite management module
//!
//! This module handles satellite components, resources, and systems for tracking
//! and managing satellite entities in the Bevy ECS.

use bevy::prelude::*;
use bevy::transform::TransformSystems;

pub mod components;
pub mod resources;
pub mod systems;

pub use components::{Satellite, SatelliteColor};
pub use resources::{
    OrbitTrailConfig, SatEntry, SatelliteRenderConfig, SatelliteStore, SelectedSatellite,
};
pub use systems::{
    draw_orbit_trails_system, move_camera_to_satellite, propagate_satellites_system,
    satellite_click_system, spawn_missing_satellite_entities_system, track_satellite_continuously,
    update_orbit_trails_system, update_satellite_rendering_system,
};

/// Plugin for satellite management and propagation
pub struct SatellitePlugin;

impl Plugin for SatellitePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SatelliteStore>()
            .init_resource::<SelectedSatellite>()
            // OrbitTrailConfig and SatelliteRenderConfig are now in UiConfigBundle
            .add_systems(
                Update,
                (
                    spawn_missing_satellite_entities_system,
                    propagate_satellites_system.after(spawn_missing_satellite_entities_system),
                    update_orbit_trails_system.after(propagate_satellites_system),
                    update_satellite_rendering_system,
                    move_camera_to_satellite,
                    track_satellite_continuously.after(propagate_satellites_system),
                    satellite_click_system,
                ),
            )
            // Draw gizmos after big_space has updated the floating origin and propagated transforms
            // for this frame. Otherwise gizmos can be a frame behind and appear to "teleport."
            .add_systems(
                PostUpdate,
                draw_orbit_trails_system.after(TransformSystems::Propagate),
            );
    }
}
