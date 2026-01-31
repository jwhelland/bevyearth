//! Satellite components for the Bevy ECS system

use bevy::math::DVec3;
use bevy::prelude::*;
use chrono::{DateTime, Utc};

/// Component marker for satellite entities
#[derive(Component)]
pub struct Satellite;

/// Component that stores the color for a satellite
#[derive(Component)]
pub struct SatelliteColor(pub Color);

/// Component that stores orbit trail history for a satellite
#[derive(Component, Default)]
pub struct OrbitTrail {
    /// Historical positions with timestamps
    pub history: Vec<TrailPoint>,
}

/// A single point in the orbit trail
#[derive(Clone)]
pub struct TrailPoint {
    /// Position in canonical ECEF km
    pub position_ecef_km: DVec3,
    /// When this point was recorded
    pub timestamp: DateTime<Utc>,
}
