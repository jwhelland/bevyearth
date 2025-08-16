//! Satellite ground track visualization
use bevy::prelude::*;

/// Plugin for ground track configuration
pub struct GroundTrackPlugin;

impl Plugin for GroundTrackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GroundTrackConfig>();
    }
}

/// Global configuration for ground track rendering
#[derive(Resource, Debug)]
pub struct GroundTrackConfig {
    /// Global enable/disable for all ground tracks
    pub enabled: bool,
    /// Radius of the ground track circle in km
    pub radius_km: f32,
}

impl Default for GroundTrackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            radius_km: 100.0,
        }
    }
}
