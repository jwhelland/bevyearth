//! Satellite systems for propagation and position updates

use crate::earth::EARTH_RADIUS_KM;
use crate::orbital::{SimulationTime, eci_to_ecef_km, gmst_rad, minutes_since_epoch};
use crate::satellite::components::{OrbitTrail, Satellite, SatelliteColor, TrailPoint};
use crate::satellite::resources::{OrbitTrailConfig, SatEcef, SatelliteStore};
use bevy::math::DVec3;
use bevy::prelude::*;

/// System to update the satellite ECEF resource from satellite transforms
pub fn update_satellite_ecef(
    sat_query: Query<&Transform, With<Satellite>>,
    mut sat_res: ResMut<SatEcef>,
) {
    if let Some(t) = sat_query.iter().next() {
        sat_res.0 = t.translation;
    }
}

/// System to propagate satellites using SGP4 and update their transforms
pub fn propagate_satellites_system(
    store: Res<SatelliteStore>,
    sim_time: Res<SimulationTime>,
    mut q: Query<(&mut Transform, &mut SatelliteColor, Entity), With<Satellite>>,
) {
    let gmst = gmst_rad(sim_time.current_utc);
    for entry in store.items.values() {
        if let (Some(tle), Some(constants)) = (&entry.tle, &entry.propagator) {
            let mins = minutes_since_epoch(sim_time.current_utc, tle.epoch_utc);
            // sgp4 2.3.0 expects MinutesSinceEpoch newtype and returns arrays
            if let Ok(state) = constants.propagate(sgp4::MinutesSinceEpoch(mins)) {
                let pos = state.position; // [f64; 3] in km (TEME)
                let eci = DVec3::new(pos[0], pos[1], pos[2]);
                let ecef = eci_to_ecef_km(eci, gmst);
                let bevy_pos = Vec3::new(ecef.y as f32, ecef.z as f32, ecef.x as f32);
                if let Some((mut t, mut c, _)) =
                    q.iter_mut().find(|(_, _, e)| Some(*e) == entry.entity)
                {
                    t.translation = bevy_pos;
                    c.0 = entry.color;
                }
            }
        }
    }
}

/// System to spawn entities for satellites that don't have them yet (e.g., from group loading)
pub fn spawn_missing_satellite_entities_system(
    mut store: ResMut<SatelliteStore>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut satellites_to_spawn = Vec::new();

    // Collect satellites that need entities
    for (norad, entry) in store.items.iter() {
        if entry.entity.is_none() && entry.tle.is_some() {
            satellites_to_spawn.push(*norad);
        }
    }

    // Spawn entities for satellites that need them
    for norad in satellites_to_spawn {
        if let Some(entry) = store.items.get_mut(&norad) {
            let mesh = Sphere::new(100.0).mesh().ico(4).unwrap();
            let entity = commands
                .spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        // base_color: entry.color,
                        emissive: entry.color.to_linear() * 20.0,
                        ..Default::default()
                    })),
                    Satellite,
                    SatelliteColor(entry.color),
                    Transform::from_xyz(EARTH_RADIUS_KM + 5000.0, 0.0, 0.0),
                ))
                .id();
            entry.entity = Some(entity);
            println!("[SPAWN] Created entity for satellite norad={}", norad);
        }
    }
}

/// System to update orbit trail history for satellites
pub fn update_orbit_trails_system(
    store: Res<SatelliteStore>,
    sim_time: Res<SimulationTime>,
    trail_config: Res<OrbitTrailConfig>,
    mut trail_query: Query<(&mut OrbitTrail, &Transform, Entity), With<Satellite>>,
    mut commands: Commands,
) {
    let current_time = sim_time.current_utc;

    for (mut trail, transform, entity) in trail_query.iter_mut() {
        // Find the satellite entry for this entity
        if let Some(entry) = store.items.values().find(|e| e.entity == Some(entity)) {
            // Only update trail if it's enabled for this satellite
            if !entry.show_trail {
                // Clear trail if disabled
                trail.history.clear();
                continue;
            }

            // Check if enough time has passed to add a new trail point
            let should_add_point = trail.history.is_empty()
                || trail
                    .history
                    .last()
                    .map(|last| {
                        current_time
                            .signed_duration_since(last.timestamp)
                            .num_milliseconds() as f32
                            / 1000.0
                            >= trail_config.update_interval_seconds
                    })
                    .unwrap_or(true);

            if should_add_point {
                // Add new trail point
                trail.history.push(TrailPoint {
                    position: transform.translation,
                    timestamp: current_time,
                });
            }

            // Remove old trail points based on age and count limits (use global config)
            let max_age_millis = (trail_config.max_age_seconds * 1000.0) as i64;
            trail.history.retain(|point| {
                current_time
                    .signed_duration_since(point.timestamp)
                    .num_milliseconds()
                    <= max_age_millis
            });

            // Limit number of points (use global config)
            if trail.history.len() > trail_config.max_points {
                let excess = trail.history.len() - trail_config.max_points;
                trail.history.drain(0..excess);
            }
        }
    }

    // Add OrbitTrail component to satellites that don't have it but need it
    for entry in store.items.values() {
        if let Some(entity) = entry.entity {
            if entry.show_trail {
                // Check if entity already has OrbitTrail component
                if trail_query.get(entity).is_err() {
                    commands.entity(entity).insert(OrbitTrail::default());
                }
            }
        }
    }
}

/// System to draw orbit trails using gizmos
pub fn draw_orbit_trails_system(
    store: Res<SatelliteStore>,
    trail_config: Res<OrbitTrailConfig>,
    trail_query: Query<(&OrbitTrail, Entity), With<Satellite>>,
    mut gizmos: Gizmos,
    sim_time: Res<SimulationTime>,
) {
    let current_time = sim_time.current_utc;

    for (trail, entity) in trail_query.iter() {
        // Find the satellite entry for this entity to get color and settings
        if let Some(entry) = store.items.values().find(|e| e.entity == Some(entity)) {
            if !entry.show_trail || trail.history.len() < 2 {
                continue;
            }

            let base_color = entry.color;

            // Draw lines between consecutive trail points
            for window in trail.history.windows(2) {
                let point1 = &window[0];
                let point2 = &window[1];

                // Calculate alpha based on age of the older point (use global config)
                let age_seconds = current_time
                    .signed_duration_since(point1.timestamp)
                    .num_milliseconds() as f32
                    / 1000.0;
                let alpha = (1.0 - (age_seconds / trail_config.max_age_seconds))
                    .max(0.1)
                    .min(1.0);

                // Create color with fade
                let trail_color = Color::srgba(
                    base_color.to_srgba().red,
                    base_color.to_srgba().green,
                    base_color.to_srgba().blue,
                    alpha,
                );

                // Draw line segment
                gizmos.line(point1.position, point2.position, trail_color);
            }
        }
    }
}
