//! Satellite systems for propagation and position updates

use bevy::prelude::*;
use bevy::math::DVec3;
use crate::satellite::components::{Satellite, SatelliteColor};
use crate::satellite::resources::{SatelliteStore, SatEcef};
use crate::orbital::{SimulationTime, gmst_rad, eci_to_ecef_km, minutes_since_epoch};
use crate::earth::EARTH_RADIUS_KM;

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
                        base_color: entry.color,
                        emissive: entry.color.to_linear(),
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