//! Satellite systems for propagation and position updates

use crate::earth::EARTH_RADIUS_KM;
use crate::orbital::{SimulationTime, eci_to_ecef_km, gmst_rad, minutes_since_epoch};
use crate::satellite::components::{OrbitTrail, Satellite, SatelliteColor, TrailPoint};
use crate::satellite::resources::{OrbitTrailConfig, SatEcef, SatelliteStore, SelectedSatellite};
use bevy::math::DVec3;
use bevy::picking::events::Click;
use bevy::picking::events::Pointer;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;

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

/// System to move camera to selected satellite with offset
pub fn move_camera_to_satellite(
    mut selected: ResMut<SelectedSatellite>,
    store: Res<SatelliteStore>,
    mut q_camera: Query<
        (&mut PanOrbitCamera, &mut Transform),
        (With<Camera3d>, Without<Satellite>),
    >,
    q_sat: Query<&Transform, With<Satellite>>,
) {
    if let Some(norad) = selected.selected.take() {
        if let Some(entry) = store.items.get(&norad) {
            if let Some(entity) = entry.entity {
                if let Ok(sat_transform) = q_sat.get(entity) {
                    let sat_pos = sat_transform.translation;

                    let dir = sat_pos.normalize();
                    let offset = 5000.0; // km
                    let new_pos = dir * (sat_pos.length() + offset);
                    let new_radius = new_pos.length();

                    // Compute pitch and yaw from direction
                    let direction = new_pos.normalize();
                    let pitch = direction.y.asin();
                    let yaw = direction.x.atan2(direction.z);

                    if let Ok((mut poc, mut cam_transform)) = q_camera.single_mut() {
                        // Force immediate camera position without smooth transition
                        poc.focus = Vec3::ZERO;

                        // Set target values first
                        poc.target_radius = new_radius;
                        poc.target_pitch = pitch;
                        poc.target_yaw = yaw;

                        // Force immediate update by setting current values too
                        poc.radius = Some(new_radius);
                        poc.pitch = Some(pitch);
                        poc.yaw = Some(yaw);

                        // Force immediate update
                        poc.force_update = true;

                        // Also directly update the camera transform as a backup
                        let camera_pos = Vec3::new(
                            new_radius * pitch.cos() * yaw.sin(),
                            new_radius * pitch.sin(),
                            new_radius * pitch.cos() * yaw.cos(),
                        );
                        cam_transform.translation = camera_pos;
                        cam_transform.look_at(Vec3::ZERO, Vec3::Y);
                    } else {
                        println!("[CAMERA] Failed to get camera");
                    }
                } else {
                    println!("[CAMERA] Failed to get satellite transform");
                }
            } else {
                println!("[CAMERA] No entity for satellite");
            }
        } else {
            println!("[CAMERA] No satellite found for norad={}", norad);
        }
        // Clear selection after processing
        selected.selected = None;
    }
}

/// System to continuously track a satellite with the camera
pub fn track_satellite_continuously(
    tracking: Res<SelectedSatellite>,
    store: Res<SatelliteStore>,
    mut q_camera: Query<
        (&mut PanOrbitCamera, &mut Transform),
        (With<Camera3d>, Without<Satellite>),
    >,
    q_sat: Query<&Transform, With<Satellite>>,
    time: Res<Time>,
) {
    // Only track if we have a tracking target
    if let Some(tracking_norad) = tracking.tracking {
        if let Some(entry) = store.items.get(&tracking_norad) {
            if let Some(entity) = entry.entity {
                if let Ok(sat_transform) = q_sat.get(entity) {
                    let sat_pos = sat_transform.translation;

                    // Calculate desired camera position with offset
                    let dir = sat_pos.normalize();
                    let offset = tracking.tracking_offset;
                    let target_pos = dir * (sat_pos.length() + offset);
                    let target_radius = target_pos.length();

                    // Compute pitch and yaw from direction
                    let direction = target_pos.normalize();
                    let target_pitch = direction.y.asin();
                    let target_yaw = direction.x.atan2(direction.z);

                    if let Ok((mut poc, mut cam_transform)) = q_camera.single_mut() {
                        // Smoothly interpolate to target position
                        let smooth_factor = tracking.smooth_factor;
                        let dt = time.delta_secs();
                        let lerp_factor = 1.0 - (1.0 - smooth_factor).powf(dt * 60.0); // 60fps normalized

                        // Update PanOrbitCamera targets
                        poc.target_radius = target_radius;
                        poc.target_pitch = target_pitch;
                        poc.target_yaw = target_yaw;
                        poc.focus = Vec3::ZERO;

                        // Smoothly update current values if they exist
                        if let Some(current_radius) = poc.radius {
                            poc.radius = Some(
                                current_radius + (target_radius - current_radius) * lerp_factor,
                            );
                        } else {
                            poc.radius = Some(target_radius);
                        }

                        if let Some(current_pitch) = poc.pitch {
                            poc.pitch =
                                Some(current_pitch + (target_pitch - current_pitch) * lerp_factor);
                        } else {
                            poc.pitch = Some(target_pitch);
                        }

                        if let Some(current_yaw) = poc.yaw {
                            // Handle yaw wrapping for shortest path
                            let mut yaw_diff = target_yaw - current_yaw;
                            if yaw_diff > std::f32::consts::PI {
                                yaw_diff -= 2.0 * std::f32::consts::PI;
                            } else if yaw_diff < -std::f32::consts::PI {
                                yaw_diff += 2.0 * std::f32::consts::PI;
                            }
                            poc.yaw = Some(current_yaw + yaw_diff * lerp_factor);
                        } else {
                            poc.yaw = Some(target_yaw);
                        }

                        // Also update transform directly for immediate visual feedback
                        let current_radius = poc.radius.unwrap_or(target_radius);
                        let current_pitch = poc.pitch.unwrap_or(target_pitch);
                        let current_yaw = poc.yaw.unwrap_or(target_yaw);

                        let camera_pos = Vec3::new(
                            current_radius * current_pitch.cos() * current_yaw.sin(),
                            current_radius * current_pitch.sin(),
                            current_radius * current_pitch.cos() * current_yaw.cos(),
                        );
                        cam_transform.translation = camera_pos;
                        cam_transform.look_at(Vec3::ZERO, Vec3::Y);
                    }
                }
            }
        }
    }
}

/// System to handle satellite click events and update the clicked satellite in the store
pub fn satellite_click_system(
    mut store: ResMut<SatelliteStore>,
    mut click_events: EventReader<Pointer<Click>>,
    satellite_query: Query<Entity, With<Satellite>>,
) {
    for event in click_events.read() {
        let clicked_entity = event.target;

        // Check if the clicked entity is a satellite
        if satellite_query.contains(clicked_entity) {
            // First, clear the clicked status from all satellites
            for entry in store.items.values_mut() {
                entry.is_clicked = false;
            }

            // Find the corresponding satellite entry by entity and mark it as clicked
            if let Some((norad, entry)) = store
                .items
                .iter_mut()
                .find(|(_, entry)| entry.entity == Some(clicked_entity))
            {
                entry.is_clicked = true;

                info!(
                    "Clicked satellite: {} (NORAD: {})",
                    entry.name.as_deref().unwrap_or("Unnamed"),
                    norad
                );
            }
        }
    }
}
