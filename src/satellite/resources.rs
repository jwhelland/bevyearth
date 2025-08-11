//! Satellite resources for managing satellite data

use bevy::prelude::*;
use std::collections::HashMap;
use crate::coverage::CoverageParameters;
use crate::tle::TleData;

/// Resource for storing satellite data and state
#[derive(Resource)]
pub struct SatelliteStore {
    pub items: HashMap<u32, SatEntry>,
    pub next_color_hue: f32,
}

impl Default for SatelliteStore {
    fn default() -> Self {
        Self {
            items: HashMap::new(),
            next_color_hue: 0.0,
        }
    }
}

/// Individual satellite entry with all associated data
pub struct SatEntry {
    pub norad: u32,
    pub name: Option<String>,
    pub color: Color,
    pub entity: Option<Entity>,
    /// Fetched TLE data
    pub tle: Option<TleData>,
    /// SGP4 propagator constants
    pub propagator: Option<sgp4::Constants>,
    /// Last error message if any
    pub error: Option<String>,
    /// Coverage footprint parameters
    #[allow(dead_code)]
    pub coverage_params: Option<CoverageParameters>,
    /// Whether to show footprint for this satellite
    pub show_footprint: bool,
    /// Whether to show orbit trail for this satellite
    pub show_trail: bool,
}

/// Resource for satellite ECEF position (in kilometers)
#[derive(Resource, Deref, DerefMut, Default)]
pub struct SatEcef(pub Vec3);

/// Resource for configuring orbit trail behavior
#[derive(Resource)]
pub struct OrbitTrailConfig {
    /// Default maximum number of trail points per satellite
    pub max_points: usize,
    /// Default maximum age of trail points in seconds
    pub max_age_seconds: f32,
    /// Minimum time between trail point updates in seconds
    pub update_interval_seconds: f32,
}

impl Default for OrbitTrailConfig {
    fn default() -> Self {
        Self {
            max_points: 100,
            max_age_seconds: 300.0, // 5 minutes
            update_interval_seconds: 2.0, // Update every 2 seconds
        }
    }
}