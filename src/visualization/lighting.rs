//! Lighting configuration systems

use bevy::prelude::*;

use crate::orbital::SunDirection;

/// Marker component for the sun directional light
#[derive(Component)]
pub struct SunLight;

/// Update sun light direction from the simulation
pub fn update_sun_light_direction(
    sun_direction: Res<SunDirection>,
    mut lights: Query<&mut Transform, With<SunLight>>,
) {
    if !sun_direction.is_changed() {
        return;
    }

    let dir = sun_direction.0.normalize_or_zero();
    if dir.length_squared() == 0.0 {
        return;
    }

    for mut transform in lights.iter_mut() {
        // Position the light far from origin in the sun direction
        // Position the light far from origin in the sun direction
        // Distance doesn't affect DirectionalLight intensity, but makes the setup clearer
        let light_distance = 150_000.0; // 150,000 km
        transform.translation = dir * light_distance;

        // Point the light back toward the origin (Earth)
        // This sets the rotation so that -Z axis points toward Earth
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}
