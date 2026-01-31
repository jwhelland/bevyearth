//! Coordinate space boundaries and canonical world position types.

use bevy::math::{DVec3, Vec3};
use bevy::prelude::*;

use crate::core::coordinates::EARTH_RADIUS_KM;

pub const EARTH_RADIUS_KM_F64: f64 = EARTH_RADIUS_KM as f64;

/// Canonical world position: standard ECEF (km, f64).
#[derive(Component, Copy, Clone, Debug, Deref, DerefMut)]
pub struct WorldEcefKm(pub DVec3);

/// Convert standard ECEF km (f64) to Bevy render km (f32).
/// Mapping: Bevy (x,y,z) = (ECEF.y, ECEF.z, ECEF.x)
pub fn ecef_to_bevy_km(ecef_km: DVec3) -> Vec3 {
    Vec3::new(ecef_km.y as f32, ecef_km.z as f32, ecef_km.x as f32)
}

/// Convert Bevy render km (f32) to standard ECEF km (f64).
/// Inverse mapping: ECEF (x,y,z) = (Bevy.z, Bevy.x, Bevy.y)
pub fn bevy_to_ecef_km(bevy_km: Vec3) -> DVec3 {
    DVec3::new(bevy_km.z as f64, bevy_km.x as f64, bevy_km.y as f64)
}
