//! Satellite systems for propagation and position updates

use crate::core::coordinates::EARTH_RADIUS_KM;
use crate::core::space::{WorldEcefKm, ecef_to_bevy_km};
use crate::orbital::{
    Dut1, SimulationTime, eci_to_ecef_km, gmst_rad_with_dut1, minutes_since_epoch,
};
use crate::satellite::components::{
    NoradId, OrbitTrail, Propagator, Satellite, SatelliteColor, SatelliteFlags, SatelliteGroupUrl,
    SatelliteName, TleComponent, TrailPoint,
};
use crate::satellite::resources::{GroupMaterialCache, GroupRegistry, SatelliteRenderAssets};
use bevy::color::LinearRgba;
use bevy::math::DVec3;
use bevy::picking::events::PointerClick;
use bevy::prelude::*;

type PropagateQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static TleComponent,
        &'static Propagator,
        &'static mut Transform,
        &'static mut SatelliteColor,
        Option<&'static mut WorldEcefKm>,
    ),
    With<Satellite>,
>;

type UnmaterializedQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static NoradId,
        &'static SatelliteColor,
        Option<&'static SatelliteGroupUrl>,
    ),
    (With<Satellite>, Without<Mesh3d>),
>;

type SatellitesWithoutTrailQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static SatelliteFlags), (With<Satellite>, Without<OrbitTrail>)>;

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
    sim_time: Res<SimulationTime>,
    dut1: Res<Dut1>,
    mut q: PropagateQuery<'_, '_>,
    mut commands: Commands,
) {
    let gmst = gmst_rad_with_dut1(sim_time.current_utc, **dut1);
    for (entity, tle_comp, propagator, mut transform, _color, world_opt) in &mut q {
        let mins = minutes_since_epoch(sim_time.current_utc, tle_comp.0.epoch_utc);
        // sgp4 2.3.0 expects MinutesSinceEpoch newtype and returns arrays
        if let Ok(state) = propagator.0.propagate(sgp4::MinutesSinceEpoch(mins)) {
            let pos = state.position; // [f64; 3] in km (TEME)
            let eci = DVec3::new(pos[0], pos[1], pos[2]);
            let ecef = eci_to_ecef_km(eci, gmst);

            transform.translation = ecef_to_bevy_km(ecef);
            if let Some(mut world) = world_opt {
                world.0 = ecef;
            } else {
                commands.entity(entity).insert(WorldEcefKm(ecef));
            }
        }
    }
}

/// System to add rendering components (mesh, material, transform) to satellite
/// entities that don't have them yet. Entities are spawned as data-only by the
/// TLE processing system; this system "materializes" them for rendering.
pub fn materialize_satellite_entities_system(
    unmaterialized: UnmaterializedQuery<'_, '_>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    render_assets: Res<SatelliteRenderAssets>,
    mut group_materials: ResMut<GroupMaterialCache>,
    config_bundle: Res<crate::ui::systems::UiConfigBundle>,
) {
    let sphere_radius = config_bundle.render_cfg.sphere_radius;
    let emissive_intensity = config_bundle.render_cfg.emissive_intensity;

    for (entity, norad_id, color, group_url_opt) in unmaterialized.iter() {
        let material_handle = if let Some(group_url) = group_url_opt {
            // Reuse or create shared group material
            if let Some(handle) = group_materials.materials.get(&group_url.0) {
                handle.clone()
            } else {
                let handle = materials.add(StandardMaterial {
                    base_color: color.0,
                    emissive: LinearRgba::from(color.0) * emissive_scale(emissive_intensity),
                    ..Default::default()
                });
                group_materials
                    .materials
                    .insert(group_url.0.clone(), handle.clone());
                handle
            }
        } else {
            // Individual material
            materials.add(StandardMaterial {
                base_color: color.0,
                emissive: LinearRgba::from(color.0) * emissive_scale(emissive_intensity),
                ..Default::default()
            })
        };

        commands.entity(entity).insert((
            Mesh3d(render_assets.sphere_mesh.clone()),
            MeshMaterial3d(material_handle),
            Transform::from_xyz(EARTH_RADIUS_KM + 5000.0, 0.0, 0.0)
                .with_scale(Vec3::splat(sphere_radius)),
            Visibility::Visible,
            Name::new(format!("Satellite {}", norad_id.0)),
        ));
    }
}

/// System to update orbit trail history for satellites
pub fn update_orbit_trails_system(
    sim_time: Res<SimulationTime>,
    config_bundle: Res<crate::ui::systems::UiConfigBundle>,
    mut trail_query: Query<(&mut OrbitTrail, &WorldEcefKm, &SatelliteFlags), With<Satellite>>,
    satellites_without_trail: SatellitesWithoutTrailQuery<'_, '_>,
    mut commands: Commands,
) {
    let current_time = sim_time.current_utc;

    for (mut trail, world_ecef, flags) in &mut trail_query {
        // Only update trail if it's enabled for this satellite
        if !flags.show_trail {
            // Clear trail if disabled
            trail.history.clear();
            continue;
        }

        // Check if enough time has passed to add a new trail point
        let should_add_point = trail.history.is_empty()
            || trail.history.last().is_none_or(|last| {
                current_time
                    .signed_duration_since(last.timestamp)
                    .num_milliseconds() as f32
                    / 1000.0
                    >= config_bundle.trail_cfg.update_interval_seconds
            });

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

    // Add OrbitTrail component to satellites that don't have it but need it
    for (entity, flags) in satellites_without_trail.iter() {
        if flags.show_trail {
            commands.entity(entity).insert(OrbitTrail::default());
        }
    }
}

/// System to draw orbit trails using gizmos
pub fn draw_orbit_trails_system(
    trail_query: Query<(&OrbitTrail, &SatelliteColor, &SatelliteFlags), With<Satellite>>,
    mut gizmos: Gizmos,
) {
    for (trail, color, flags) in trail_query.iter() {
        if !flags.show_trail || trail.history.len() < 2 {
            continue;
        }

        let srgba = color.0.to_srgba();
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

/// System to handle satellite click events and update the clicked satellite in the store
pub fn satellite_click_system(
    mut click_events: MessageReader<PointerClick>,
    norad_query: Query<(&NoradId, Option<&SatelliteName>), With<Satellite>>,
    mut all_satellites: Query<&mut SatelliteFlags, With<Satellite>>,
) {
    for event in click_events.read() {
        let clicked_entity = event.entity;

        // Check if the clicked entity is a satellite
        if let Ok((norad, name_opt)) = norad_query.get(clicked_entity) {
            // First, clear the clicked status from all satellites
            for mut flags in &mut all_satellites {
                flags.is_clicked = false;
            }

            // Set clicked status on the clicked satellite
            if let Ok(mut flags) = all_satellites.get_mut(clicked_entity) {
                flags.is_clicked = true;

                info!(
                    "Clicked satellite: {} (NORAD: {})",
                    name_opt.map_or("Unnamed", |n| n.0.as_str()),
                    norad.0
                );
            }
        }
    }
}

/// System to update satellite rendering properties when config changes
pub fn update_satellite_rendering_system(
    config_bundle: Res<crate::ui::systems::UiConfigBundle>,
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
        if let Some(mut material) = materials.get_mut(handle) {
            material.emissive = LinearRgba::from(material.base_color)
                * emissive_scale(config_bundle.render_cfg.emissive_intensity);
        }
    }

    for (mut transform, satellite_color, entity) in &mut satellite_query {
        // Update scale based on sphere_radius
        transform.scale = Vec3::splat(config_bundle.render_cfg.sphere_radius);

        // Update material emissive intensity and base color
        if let Ok(material_handle) = material_query.get(entity)
            && let Some(mut material) = materials.get_mut(&material_handle.0)
        {
            material.base_color = satellite_color.0;
            material.emissive = LinearRgba::from(satellite_color.0)
                * emissive_scale(config_bundle.render_cfg.emissive_intensity);
        }
    }
}

/// System to propagate group color changes to all satellites in affected groups
///
/// This system runs when the `GroupRegistry` is changed (e.g., via UI color picker)
/// and updates the color of all satellites belonging to the modified group.
pub fn update_group_colors_system(
    group_registry: Option<Res<GroupRegistry>>,
    config_bundle: Res<crate::ui::systems::UiConfigBundle>,
    mut satellite_query: Query<(&mut SatelliteColor, &SatelliteGroupUrl, Entity), With<Satellite>>,
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
    for (group_url, group) in &registry.groups {
        if let Some(handle) = group_materials.materials.get(group_url)
            && let Some(mut material) = materials.get_mut(handle)
        {
            material.base_color = group.color;
            material.emissive = LinearRgba::from(group.color)
                * emissive_scale(config_bundle.render_cfg.emissive_intensity);
        }
    }

    // Update colors for satellites that belong to groups
    for (mut satellite_color, group_url, entity) in &mut satellite_query {
        if let Some(group) = registry.groups.get(&group_url.0) {
            satellite_color.0 = group.color;

            // Update the material for non-group-cached satellites
            if !group_materials.materials.contains_key(&group_url.0)
                && let Ok(material_handle) = material_query.get(entity)
                && let Some(mut material) = materials.get_mut(&material_handle.0)
            {
                material.base_color = group.color;
                material.emissive = LinearRgba::from(group.color)
                    * emissive_scale(config_bundle.render_cfg.emissive_intensity);
            }
        }
    }
}
