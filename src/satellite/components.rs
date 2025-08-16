//! Satellite components for the Bevy ECS system

use bevy::prelude::*;
use chrono::{DateTime, Utc};

/// Component marker for satellite entities
#[derive(Component)]
pub struct Satellite;

/// Component that stores the color for a satellite
#[derive(Component)]
pub struct SatelliteColor(pub Color);

/// Component that stores orbit trail history for a satellite
#[derive(Component)]
pub struct OrbitTrail {
    /// Historical positions with timestamps
    pub history: Vec<TrailPoint>,
}

/// A single point in the orbit trail
#[derive(Clone)]
pub struct TrailPoint {
    /// Position in world space
    pub position: Vec3,
    /// When this point was recorded
    pub timestamp: DateTime<Utc>,
}

impl Default for OrbitTrail {
    fn default() -> Self {
        Self {
            history: Vec::new(),
        }
    }
}
