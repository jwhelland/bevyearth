//! Satellite resources for managing satellite data

use bevy::prelude::*;
use crate::coverage::CoverageParameters;
use crate::tle::TleData;

/// Resource for storing satellite data and state
#[derive(Resource, Default)]
pub struct SatelliteStore {
    pub items: Vec<SatEntry>,
    pub next_color_hue: f32,
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
    pub coverage_params: Option<CoverageParameters>,
    /// Whether to show footprint for this satellite
    pub show_footprint: bool,
}

/// Resource for satellite ECEF position (in kilometers)
#[derive(Resource, Deref, DerefMut, Default)]
pub struct SatEcef(pub Vec3);