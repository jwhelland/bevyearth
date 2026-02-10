//! Satellite systems for propagation and position updates

use crate::core::coordinates::EARTH_RADIUS_KM;
use crate::core::space::{WorldEcefKm, ecef_to_bevy_km};
use crate::orbital::{
    Dut1, SimulationTime, eci_to_ecef_km, gmst_rad_with_dut1, minutes_since_epoch,
};
use crate::satellite::components::{NoradId, OrbitTrail, Satellite, SatelliteColor, TrailPoint};
use crate::satellite::resources::{
    GroupMaterialCache, GroupRegistry, SatelliteRenderAssets, SatelliteStore, SelectedSatellite,
};
use bevy::color::LinearRgba;
use bevy::math::DVec3;
use bevy::picking::events::Click;
use bevy::picking::events::Pointer;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;

type CameraQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut PanOrbitCamera, &'static mut Transform),
    (With<Camera3d>, Without<Satellite>),
>;

fn emissive_scale(intensity: f32) -> f32 {
    // Square for perceptual control: low values have more visible effect, high values still pop.
    intensity * intensity
}

/// Create shared satellite render assets
pub fn init_satellite_render_assets(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let mesh = Sphere::new(1.0).mesh().ico(4).unwrap();
    commands.insert_resource(SatelliteRenderAssets {
        sphere_mesh: meshes.add(mesh),
    });
}

/// System to propagate satellites using SGP4 and update their transforms
pub fn propagate_satellites_system(
    store: Res<SatelliteStore>,
    sim_time: Res<SimulationTime>,
    dut1: Res<Dut1>,
    mut q: Query<
        (
            &mut Transform,
            &mut SatelliteColor,
            Option<&mut WorldEcefKm>,
        ),
        With<Satellite>,
    >,
    mut commands: Commands,
) {
    let gmst = gmst_rad_with_dut1(sim_time.current_utc, **dut1);
    for entry in store.items.values() {
        if let (Some(tle), Some(constants)) = (&entry.tle, &entry.propagator) {
            let mins = minutes_since_epoch(sim_time.current_utc, tle.epoch_utc);
            // sgp4 2.3.0 expects MinutesSinceEpoch newtype and returns arrays
            if let Ok(state) = constants.propagate(sgp4::MinutesSinceEpoch(mins)) {
                let pos = state.position; // [f64; 3] in km (TEME)
                let eci = DVec3::new(pos[0], pos[1], pos[2]);
                let ecef = eci_to_ecef_km(eci, gmst);
                if let Some(entity) = entry.entity
                    && let Ok((mut t, mut c, world_opt)) = q.get_mut(entity)
                {
                    t.translation = ecef_to_bevy_km(ecef);
                    if let Some(mut world) = world_opt {
                        world.0 = ecef;
                    } else {
                        commands.entity(entity).insert(WorldEcefKm(ecef));
                    }
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
    mut materials: ResMut<Assets<StandardMaterial>>,
    render_assets: Res<SatelliteRenderAssets>,
    mut group_materials: ResMut<GroupMaterialCache>,
    config_bundle: Res<crate::ui::systems::UiConfigBundle>,
) {
    let mut group_spawn_ids = Vec::new();
    let mut solo_spawn_ids = Vec::new();

    // Collect satellites that need entities
    for (norad, entry) in store.items.iter() {
        if entry.entity.is_none() {
            if entry.group_url.is_some() {
                group_spawn_ids.push(*norad);
            } else {
                solo_spawn_ids.push(*norad);
            }
        }
    }

    if !group_spawn_ids.is_empty() {
        group_spawn_ids.sort_unstable();
        let mut bundles = Vec::with_capacity(group_spawn_ids.len());
        let mesh_handle = render_assets.sphere_mesh.clone();
        let sphere_radius = config_bundle.render_cfg.sphere_radius;
        let emissive_intensity = config_bundle.render_cfg.emissive_intensity;

        for norad in group_spawn_ids.iter() {
            if let Some(entry) = store.items.get(norad) {
                let group_url = entry
                    .group_url
                    .as_ref()
                    .expect("group_url should be present for group spawn");
                let material_handle = if let Some(handle) = group_materials.materials.get(group_url)
                {
                    handle.clone()
                } else {
                    let handle = materials.add(StandardMaterial {
                        base_color: entry.color,
                        emissive: LinearRgba::from(entry.color)
                            * emissive_scale(emissive_intensity),
                        ..Default::default()
                    });
                    group_materials
                        .materials
                        .insert(group_url.clone(), handle.clone());
                    handle
                };

                bundles.push((
                    Mesh3d(mesh_handle.clone()),
                    MeshMaterial3d(material_handle),
                    NoradId(*norad),
                    Satellite,
                    SatelliteColor(entry.color),
                    Transform::from_xyz(EARTH_RADIUS_KM + 5000.0, 0.0, 0.0)
                        .with_scale(Vec3::splat(sphere_radius)),
                    Visibility::Visible,
                    Name::new(format!("Satellite {norad}")),
                ));
            }
        }

        let group_ids = group_spawn_ids.clone();
        let batch_count = group_ids.len();
        commands.queue(move |world: &mut World| {
            let entities: Vec<Entity> = world.spawn_batch(bundles).collect();
            let mut store = world.resource_mut::<SatelliteStore>();
            for (norad, entity) in group_ids.into_iter().zip(entities) {
                if let Some(entry) = store.items.get_mut(&norad) {
                    entry.entity = Some(entity);
                }
            }

            info!(
                "Spawned {} group satellites via spawn_batch",
                batch_count
            );
        });
    }

    // Spawn entities for non-group satellites individually.
    solo_spawn_ids.sort_unstable();
    for norad in solo_spawn_ids {
        if let Some(entry) = store.items.get_mut(&norad) {
            let material_handle = materials.add(StandardMaterial {
                base_color: entry.color,
                emissive: LinearRgba::from(entry.color)
                    * emissive_scale(config_bundle.render_cfg.emissive_intensity),
                ..Default::default()
            });

            let entity = commands
                .spawn((
                    Mesh3d(render_assets.sphere_mesh.clone()),
                    MeshMaterial3d(material_handle),
                    NoradId(norad),
                    Satellite,
                    SatelliteColor(entry.color),
                    Transform::from_xyz(EARTH_RADIUS_KM + 5000.0, 0.0, 0.0)
                        .with_scale(Vec3::splat(config_bundle.render_cfg.sphere_radius)),
                    Visibility::Visible,
                    Name::new(format!("Satellite {norad}")),
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
    config_bundle: Res<crate::ui::systems::UiConfigBundle>,
    mut trail_query: Query<(&mut OrbitTrail, &WorldEcefKm, &NoradId), With<Satellite>>,
    mut commands: Commands,
) {
    let current_time = sim_time.current_utc;

    for (mut trail, world_ecef, norad) in trail_query.iter_mut() {
        if let Some(entry) = store.items.get(&norad.0) {
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
                            >= config_bundle.trail_cfg.update_interval_seconds
                    })
                    .unwrap_or(true);

            if should_add_point {
                // Add new trail point
                trail.history.push(TrailPoint {
                    position_ecef_km: world_ecef.0,
                    timestamp: current_time,
                });
            }

            // Limit number of points to max 1000
            if trail.history.len() > config_bundle.trail_cfg.max_points {
                let excess = trail.history.len() - config_bundle.trail_cfg.max_points;
                trail.history.drain(0..excess);
            }
        }
    }

    // Add OrbitTrail component to satellites that don't have it but need it
    for entry in store.items.values() {
        if let Some(entity) = entry.entity
            && entry.show_trail
            && trail_query.get(entity).is_err()
        {
            // Check if entity already has OrbitTrail component
            commands.entity(entity).insert(OrbitTrail::default());
        }
    }
}

/// System to draw orbit trails using gizmos
pub fn draw_orbit_trails_system(
    store: Res<SatelliteStore>,
    trail_query: Query<(&OrbitTrail, &NoradId), With<Satellite>>,
    mut gizmos: Gizmos,
) {
    for (trail, norad) in trail_query.iter() {
        if let Some(entry) = store.items.get(&norad.0) {
            if !entry.show_trail || trail.history.len() < 2 {
                continue;
            }

            let srgba = entry.color.to_srgba();
            let trail_length = trail.history.len() as f32;

            // Draw lines between consecutive trail points
            for (i, window) in trail.history.windows(2).enumerate() {
                let point1 = &window[0];
                let point2 = &window[1];

                // Calculate alpha based on position in trail (newer = more opaque)
                let alpha = (0.1 + 0.9 * (i as f32 / trail_length.max(1.0))).min(1.0);

                let trail_color = Color::srgba(srgba.red, srgba.green, srgba.blue, alpha);

                // Draw line segment (convert canonical ECEF to Bevy render space)
                let point1_bevy = ecef_to_bevy_km(point1.position_ecef_km);
                let point2_bevy = ecef_to_bevy_km(point2.position_ecef_km);
                gizmos.line(point1_bevy, point2_bevy, trail_color);
            }
        }
    }
}

/// System to move camera to selected satellite with offset
pub fn move_camera_to_satellite(
    mut selected: ResMut<SelectedSatellite>,
    store: Res<SatelliteStore>,
    mut q_camera: CameraQuery<'_, '_>,
    q_sat: Query<&Transform, With<Satellite>>,
) {
    if let Some(norad) = selected.selected.take() {
        if let Some(entity) = store.items.get(&norad).and_then(|e| e.entity) {
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
    mut q_camera: CameraQuery<'_, '_>,
    q_sat: Query<&Transform, With<Satellite>>,
    time: Res<Time>,
) {
    // Only track if we have a tracking target
    if let Some(tracking_norad) = tracking.tracking
        && let Some(entity) = store.items.get(&tracking_norad).and_then(|e| e.entity)
        && let Ok(sat_transform) = q_sat.get(entity)
    {
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
                poc.radius = Some(current_radius + (target_radius - current_radius) * lerp_factor);
            } else {
                poc.radius = Some(target_radius);
            }

            if let Some(current_pitch) = poc.pitch {
                poc.pitch = Some(current_pitch + (target_pitch - current_pitch) * lerp_factor);
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

/// System to handle satellite click events and update the clicked satellite in the store
pub fn satellite_click_system(
    mut store: ResMut<SatelliteStore>,
    mut click_events: MessageReader<Pointer<Click>>,
    norad_query: Query<&NoradId, With<Satellite>>,
) {
    for event in click_events.read() {
        let clicked_entity = event.entity;

        // Check if the clicked entity is a satellite
        if let Ok(norad) = norad_query.get(clicked_entity) {
            // First, clear the clicked status from all satellites
            for entry in store.items.values_mut() {
                entry.is_clicked = false;
            }

            if let Some(entry) = store.items.get_mut(&norad.0) {
                entry.is_clicked = true;

                info!(
                    "Clicked satellite: {} (NORAD: {})",
                    entry.name.as_deref().unwrap_or("Unnamed"),
                    norad.0
                );
            }
        }
    }
}

/// System to update satellite rendering properties when config changes
pub fn update_satellite_rendering_system(
    config_bundle: Res<crate::ui::systems::UiConfigBundle>,
    _store: Res<SatelliteStore>,
    mut satellite_query: Query<(&mut Transform, &SatelliteColor, Entity), With<Satellite>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    material_query: Query<&MeshMaterial3d<StandardMaterial>>,
    group_materials: Res<GroupMaterialCache>,
) {
    // Only update if the config has changed
    if !config_bundle.is_changed() {
        return;
    }

    for handle in group_materials.materials.values() {
        if let Some(material) = materials.get_mut(handle) {
            material.emissive = LinearRgba::from(material.base_color)
                * emissive_scale(config_bundle.render_cfg.emissive_intensity);
        }
    }

    for (mut transform, satellite_color, entity) in satellite_query.iter_mut() {
        // Update scale based on sphere_radius
        transform.scale = Vec3::splat(config_bundle.render_cfg.sphere_radius);

        // Update material emissive intensity and base color
        if let Ok(material_handle) = material_query.get(entity)
            && let Some(material) = materials.get_mut(&material_handle.0)
        {
            material.base_color = satellite_color.0;
            material.emissive = LinearRgba::from(satellite_color.0)
                * emissive_scale(config_bundle.render_cfg.emissive_intensity);
        }
    }
}

/// System to propagate group color changes to all satellites in affected groups
///
/// This system runs when the GroupRegistry is changed (e.g., via UI color picker)
/// and updates the color of all satellites belonging to the modified group.
pub fn update_group_colors_system(
    group_registry: Option<Res<GroupRegistry>>,
    mut store: ResMut<SatelliteStore>,
    config_bundle: Res<crate::ui::systems::UiConfigBundle>,
    mut satellite_query: Query<(&NoradId, &mut SatelliteColor, Entity), With<Satellite>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    material_query: Query<&MeshMaterial3d<StandardMaterial>>,
    group_materials: Res<GroupMaterialCache>,
) {
    // Only proceed if GroupRegistry exists and has changed
    let Some(registry) = group_registry else {
        return;
    };
    if !registry.is_changed() {
        return;
    }

    // Update cached group materials first (if any exist).
    for (group_url, group) in registry.groups.iter() {
        if let Some(handle) = group_materials.materials.get(group_url)
            && let Some(material) = materials.get_mut(handle)
        {
            material.base_color = group.color;
            material.emissive = LinearRgba::from(group.color)
                * emissive_scale(config_bundle.render_cfg.emissive_intensity);
        }
    }

    // Update colors for satellites that belong to groups
    for (norad_id, mut satellite_color, entity) in satellite_query.iter_mut() {
        if let Some(entry) = store.items.get_mut(&norad_id.0) {
            // Check if this satellite belongs to a group
            if let Some(group_url) = &entry.group_url {
                // Look up the group's current color
                if let Some(group) = registry.groups.get(group_url) {
                    // Update the color in all relevant places
                    entry.color = group.color;
                    satellite_color.0 = group.color;

                    // Update the material
                    if !group_materials.materials.contains_key(group_url)
                        && let Ok(material_handle) = material_query.get(entity)
                        && let Some(material) = materials.get_mut(&material_handle.0)
                    {
                        material.base_color = group.color;
                        material.emissive = LinearRgba::from(group.color)
                            * emissive_scale(config_bundle.render_cfg.emissive_intensity);
                    }
                }
            }
        }
    }
}
