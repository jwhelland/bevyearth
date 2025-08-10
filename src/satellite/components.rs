//! Satellite components for the Bevy ECS system

use bevy::prelude::*;

/// Component marker for satellite entities
#[derive(Component)]
pub struct Satellite;

/// Component that stores the color for a satellite
#[derive(Component)]
pub struct SatelliteColor(pub Color);