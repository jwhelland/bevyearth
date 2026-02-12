//! Satellite components for the Bevy ECS system

use crate::tle::TleData;
use bevy::math::DVec3;
use bevy::prelude::*;
use chrono::{DateTime, Utc};

/// Component marker for satellite entities
#[derive(Component)]
pub struct Satellite;

/// Component storing NORAD ID for a satellite entity
#[derive(Component, Copy, Clone, Debug)]
pub struct NoradId(pub u32);

/// Component that stores the color for a satellite
#[derive(Component)]
pub struct SatelliteColor(pub Color);

/// Component storing the satellite's name
#[derive(Component)]
pub struct SatelliteName(pub String);

/// Component storing TLE orbital data
#[derive(Component)]
pub struct TleComponent(pub TleData);

/// Component storing SGP4 propagator constants
#[derive(Component)]
pub struct Propagator(pub sgp4::Constants);

/// Component storing propagation error message
#[derive(Component)]
pub struct PropagationError(pub String);

/// Component storing the group URL this satellite belongs to
#[derive(Component)]
pub struct SatelliteGroupUrl(pub String);

/// Component storing satellite visualization flags
#[derive(Component, Default)]
pub struct SatelliteFlags {
    pub show_ground_track: bool,
    pub show_trail: bool,
    pub is_clicked: bool,
}

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
