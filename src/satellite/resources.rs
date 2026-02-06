//! Satellite resources for managing satellite data

use crate::tle::TleData;
use bevy::prelude::*;
use std::collections::HashMap;

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
    /// Whether to show ground track for this satellite
    pub show_ground_track: bool,
    /// Whether to show orbit trail for this satellite
    pub show_trail: bool,
    /// Whether this satellite is currently clicked/selected for display
    pub is_clicked: bool,
}

/// Resource for configuring orbit trail behavior
#[derive(Resource)]
pub struct OrbitTrailConfig {
    /// Maximum number of trail points per satellite
    pub max_points: usize,
    /// Minimum time between trail point updates in seconds
    pub update_interval_seconds: f32,
}

impl Default for OrbitTrailConfig {
    fn default() -> Self {
        Self {
            max_points: 1000,
            update_interval_seconds: 2.0, // Update every 2 seconds
        }
    }
}
/// Resource for tracking the selected satellite for camera focus
#[derive(Resource)]
pub struct SelectedSatellite {
    /// One-time camera movement to satellite (existing behavior)
    pub selected: Option<u32>,
    /// Continuous camera tracking of satellite (new behavior)
    pub tracking: Option<u32>,
    /// Distance offset from satellite for tracking camera (in km)
    pub tracking_offset: f32,
    /// Smooth interpolation factor for camera movement (0.0 to 1.0)
    pub smooth_factor: f32,
}

impl Default for SelectedSatellite {
    fn default() -> Self {
        Self {
            selected: None,
            tracking: None,
            tracking_offset: 5000.0, // 5000 km default offset
            smooth_factor: 0.1,      // Smooth camera movement
        }
    }
}

/// Resource for configuring satellite rendering properties
#[derive(Resource)]
pub struct SatelliteRenderConfig {
    /// Radius of satellite spheres in kilometers
    pub sphere_radius: f32,
    /// Emissive intensity multiplier for satellite materials
    pub emissive_intensity: f32,
}

impl Default for SatelliteRenderConfig {
    fn default() -> Self {
        Self {
            sphere_radius: 100.0,
            emissive_intensity: 20.0,
        }
    }
}

/// Shared render assets for satellites
#[derive(Resource, Clone)]
pub struct SatelliteRenderAssets {
    pub sphere_mesh: Handle<Mesh>,
}
