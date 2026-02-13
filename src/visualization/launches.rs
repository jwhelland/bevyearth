//! Launch pad visualization (markers).

use crate::core::coordinates::Coordinates;
use crate::core::space::{WorldEcefKm, ecef_to_bevy_km};
use crate::launch_library::{LaunchLibraryConfig, LaunchLibraryData, LaunchSummary};
use crate::ui::state::{LaunchLibraryItemKind, LaunchLibrarySelection, LaunchLibraryUiState};
use bevy::math::DVec3;
use bevy::mesh::{
    ConeAnchor, ConeMeshBuilder, CylinderAnchor, CylinderMeshBuilder, TorusMeshBuilder,
};
use bevy::picking::events::{Click, Pointer};
use bevy::prelude::ChildOf;
use bevy::prelude::*;
use bevy::prelude::AlphaMode;
use chrono::DateTime;
use chrono::Utc;
use std::collections::HashMap;

const ROCKET_BODY_HEIGHT: f32 = 60.0;
const ROCKET_BODY_RADIUS: f32 = 6.5;
const ROCKET_NOSE_HEIGHT: f32 = 18.0;
const ROCKET_RING_INNER: f32 = 16.0;
const ROCKET_RING_OUTER: f32 = 19.0;
const ROCKET_RING2_INNER: f32 = 22.0;
const ROCKET_RING2_OUTER: f32 = 26.0;
const ROCKET_RING_OFFSET_Y: f32 = 2.0;
const ROCKET_GLOW_RADIUS: f32 = 12.0;
const ROCKET_GLOW_HEIGHT: f32 = 70.0;
const ROCKET_SURFACE_OFFSET_KM: f32 = 1.5;

#[derive(Component, Clone)]
#[allow(dead_code)]
pub struct LaunchPadMarker {
    pub pad_key: String,
    pub pad_id: Option<i64>,
    pub pad_name: String,
    pub pad_lat: f64,
    pub pad_lon: f64,
    pub launch_count: usize,
    pub next_net: Option<DateTime<Utc>>,
}

#[derive(Resource)]
struct LaunchPadAssets {
    body_mesh: Handle<Mesh>,
    nose_mesh: Handle<Mesh>,
    ring_mesh: Handle<Mesh>,
    ring2_mesh: Handle<Mesh>,
    glow_mesh: Handle<Mesh>,
    body_material: Handle<StandardMaterial>,
    nose_material: Handle<StandardMaterial>,
    ring_material: Handle<StandardMaterial>,
    ring2_material: Handle<StandardMaterial>,
    glow_material: Handle<StandardMaterial>,
}

#[derive(Component, Clone, Copy)]
struct PulseRing {
    base_scale: f32,
    speed: f32,
    amplitude: f32,
    phase: f32,
}

pub struct LaunchesPlugin;

impl Plugin for LaunchesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_launch_pad_assets)
            .add_systems(
                Update,
                (
                    update_launch_pad_markers,
                    sync_launch_pad_visibility,
                    animate_pulse_rings,
                    handle_launch_pad_clicks,
                )
                    .chain(),
            );
    }
}

fn setup_launch_pad_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let body_mesh = meshes.add(
        CylinderMeshBuilder::new(ROCKET_BODY_RADIUS, ROCKET_BODY_HEIGHT, 24)
            .anchor(CylinderAnchor::Bottom),
    );
    let nose_mesh = meshes.add(
        ConeMeshBuilder::new(ROCKET_BODY_RADIUS, ROCKET_NOSE_HEIGHT, 24).anchor(ConeAnchor::Base),
    );
    let ring_mesh = meshes.add(
        TorusMeshBuilder::new(ROCKET_RING_INNER, ROCKET_RING_OUTER)
            .major_resolution(48)
            .minor_resolution(16),
    );
    let ring2_mesh = meshes.add(
        TorusMeshBuilder::new(ROCKET_RING2_INNER, ROCKET_RING2_OUTER)
            .major_resolution(56)
            .minor_resolution(16),
    );
    let glow_mesh = meshes.add(
        ConeMeshBuilder::new(ROCKET_GLOW_RADIUS, ROCKET_GLOW_HEIGHT, 24).anchor(ConeAnchor::Base),
    );

    let body_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.4, 0.1),
        emissive: LinearRgba::new(1.2, 0.5, 0.2, 1.0),
        unlit: true,
        ..default()
    });
    let nose_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.98),
        emissive: LinearRgba::new(0.4, 0.4, 0.5, 1.0),
        unlit: true,
        ..default()
    });
    let ring_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.8, 1.0, 0.7),
        emissive: LinearRgba::new(1.0, 1.6, 2.2, 1.0),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });
    let ring2_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.6, 1.0, 0.5),
        emissive: LinearRgba::new(0.6, 1.0, 1.8, 1.0),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });
    let glow_material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::new(0.9, 0.6, 0.2, 1.0),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });

    commands.insert_resource(LaunchPadAssets {
        body_mesh,
        nose_mesh,
        ring_mesh,
        ring2_mesh,
        glow_mesh,
        body_material,
        nose_material,
        ring_material,
        ring2_material,
        glow_material,
    });
}

fn update_launch_pad_markers(
    data: Res<LaunchLibraryData>,
    config: Res<LaunchLibraryConfig>,
    assets: Res<LaunchPadAssets>,
    mut query: Query<(Entity, &LaunchPadMarker, &mut Transform, &mut Visibility)>,
    mut commands: Commands,
) {
    if !data.is_changed() && !config.is_changed() {
        return;
    }

    let pad_markers = build_pad_markers(&data.launches);
    let mut existing: HashMap<String, Entity> = HashMap::new();
    for (entity, marker, _transform, _visibility) in query.iter_mut() {
        existing.insert(marker.pad_key.clone(), entity);
    }

    let show_markers = config.show_pad_markers;

    for marker in pad_markers {
        let Some(ecef) = pad_ecef_from_marker(&marker) else {
            warn!(
                "Invalid coordinates for launch pad: {} ({}, {})",
                marker.pad_name, marker.pad_lat, marker.pad_lon
            );
            continue;
        };
        let bevy_pos = ecef_to_bevy_km(ecef);
        let transform = marker_transform(bevy_pos, marker.launch_count);

        let phase = hash_phase(&marker.pad_key);

        if let Some(entity) = existing.remove(&marker.pad_key) {
            commands.entity(entity).insert((
                marker,
                transform,
                WorldEcefKm(ecef),
                if show_markers {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                },
            ));
        } else {
            let marker_entity = commands
                .spawn((
                    Transform::from_translation(transform.translation)
                        .with_rotation(transform.rotation)
                        .with_scale(transform.scale),
                    WorldEcefKm(ecef),
                    if show_markers {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    },
                    marker,
                ))
                .id();

            commands.entity(marker_entity).with_children(|parent| {
                parent.spawn((
                    Mesh3d(assets.body_mesh.clone()),
                    MeshMaterial3d(assets.body_material.clone()),
                    Transform::from_translation(Vec3::ZERO),
                    Pickable::default(),
                ));
                parent.spawn((
                    Mesh3d(assets.nose_mesh.clone()),
                    MeshMaterial3d(assets.nose_material.clone()),
                    Transform::from_translation(Vec3::new(0.0, ROCKET_BODY_HEIGHT, 0.0)),
                    Pickable::default(),
                ));
                parent.spawn((
                    Mesh3d(assets.glow_mesh.clone()),
                    MeshMaterial3d(assets.glow_material.clone()),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                ));
                parent.spawn((
                    Mesh3d(assets.ring_mesh.clone()),
                    MeshMaterial3d(assets.ring_material.clone()),
                    Transform::from_translation(Vec3::new(0.0, ROCKET_RING_OFFSET_Y, 0.0)),
                    PulseRing {
                        base_scale: 1.0,
                        speed: 1.1,
                        amplitude: 0.35,
                        phase,
                    },
                    Visibility::Visible,
                ));
                parent.spawn((
                    Mesh3d(assets.ring2_mesh.clone()),
                    MeshMaterial3d(assets.ring2_material.clone()),
                    Transform::from_translation(Vec3::new(0.0, ROCKET_RING_OFFSET_Y, 0.0)),
                    PulseRing {
                        base_scale: 1.0,
                        speed: 0.6,
                        amplitude: 0.18,
                        phase: phase + 1.2,
                    },
                    Visibility::Visible,
                ));
            });
        }
    }

    for (_key, entity) in existing {
        commands.entity(entity).despawn_children();
        commands.entity(entity).despawn();
    }

    if !show_markers {
        for (entity, _marker, _transform, _visibility) in query.iter_mut() {
            commands.entity(entity).insert(Visibility::Hidden);
        }
    }
}

fn sync_launch_pad_visibility(
    config: Res<LaunchLibraryConfig>,
    mut query: Query<&mut Visibility, With<LaunchPadMarker>>,
) {
    if !config.is_changed() {
        return;
    }

    let visibility = if config.show_pad_markers {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for mut vis in query.iter_mut() {
        *vis = visibility;
    }
}

fn marker_scale(count: usize) -> f32 {
    let bump = (count.saturating_sub(1) as f32 * 0.15).clamp(0.0, 0.6);
    1.0 + bump
}

fn marker_transform(bevy_pos: Vec3, count: usize) -> Transform {
    let mut normal = bevy_pos.normalize_or_zero();
    if normal.length_squared() < 1e-6 {
        normal = Vec3::Y;
    }
    let rotation = Quat::from_rotation_arc(Vec3::Y, normal);
    let translation = bevy_pos + normal * ROCKET_SURFACE_OFFSET_KM;
    let scale = Vec3::splat(marker_scale(count));
    Transform {
        translation,
        rotation,
        scale,
    }
}

fn hash_phase(key: &str) -> f32 {
    let mut hash: u32 = 2166136261;
    for b in key.as_bytes() {
        hash ^= *b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    (hash as f32 / u32::MAX as f32) * std::f32::consts::TAU
}

fn animate_pulse_rings(time: Res<Time>, mut rings: Query<(&PulseRing, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (ring, mut transform) in rings.iter_mut() {
        let pulse = 1.0 + ring.amplitude * (t * ring.speed + ring.phase).sin();
        transform.scale = Vec3::splat(ring.base_scale * pulse);
    }
}

fn handle_launch_pad_clicks(
    mut click_events: MessageReader<Pointer<Click>>,
    markers: Query<&LaunchPadMarker>,
    parents: Query<&ChildOf>,
    data: Res<LaunchLibraryData>,
    mut launch_ui: ResMut<LaunchLibraryUiState>,
) {
    if data.launches.is_empty() {
        return;
    }

    for ev in click_events.read() {
        let mut entity = ev.entity;
        let marker = loop {
            if let Ok(marker) = markers.get(entity) {
                break Some(marker);
            }
            if let Ok(parent) = parents.get(entity) {
                entity = parent.parent();
                continue;
            }
            break None;
        };

        let Some(marker) = marker else { continue };
        if let Some(index) = find_launch_index_for_marker(marker, &data.launches) {
            launch_ui.selection = Some(LaunchLibrarySelection {
                kind: LaunchLibraryItemKind::Launch,
                index,
            });
        }
    }
}

fn find_launch_index_for_marker(
    marker: &LaunchPadMarker,
    launches: &[LaunchSummary],
) -> Option<usize> {
    let mut best: Option<(usize, DateTime<Utc>)> = None;

    for (idx, launch) in launches.iter().enumerate() {
        let matches_id = marker.pad_id.is_some()
            && launch.pad_id.is_some()
            && marker.pad_id == launch.pad_id;
        let matches_coords = launch.pad_lat.is_some()
            && launch.pad_lon.is_some()
            && ((launch.pad_lat.unwrap() - marker.pad_lat).abs() < 0.01)
            && ((launch.pad_lon.unwrap() - marker.pad_lon).abs() < 0.01);

        if !(matches_id || matches_coords) {
            continue;
        }

        if let Some(net) = launch.net_utc {
            match best {
                Some((_, best_net)) if net >= best_net => {}
                _ => best = Some((idx, net)),
            }
        } else if best.is_none() {
            best = Some((idx, Utc::now()));
        }
    }

    best.map(|(idx, _)| idx)
}

fn build_pad_markers(launches: &[LaunchSummary]) -> Vec<LaunchPadMarker> {
    let mut map: HashMap<String, LaunchPadMarker> = HashMap::new();

    for launch in launches {
        let (Some(lat), Some(lon)) = (launch.pad_lat, launch.pad_lon) else {
            continue;
        };
        let pad_name = launch
            .pad_name
            .clone()
            .unwrap_or_else(|| "Launch Pad".to_string());
        let pad_key = launch
            .pad_id
            .map(|id| format!("id:{id}"))
            .unwrap_or_else(|| format!("name:{}:{:.3}:{:.3}", pad_name, lat, lon));

        let entry = map.entry(pad_key.clone()).or_insert_with(|| LaunchPadMarker {
            pad_key: pad_key.clone(),
            pad_id: launch.pad_id,
            pad_name: pad_name.clone(),
            pad_lat: lat,
            pad_lon: lon,
            launch_count: 0,
            next_net: launch.net_utc,
        });
        entry.launch_count += 1;
        if let Some(net) = launch.net_utc {
            let next = entry.next_net;
            if next.is_none() || next.is_some_and(|t| net < t) {
                entry.next_net = Some(net);
            }
        }
    }

    map.into_values().collect()
}

fn pad_ecef_from_marker(marker: &LaunchPadMarker) -> Option<DVec3> {
    Coordinates::from_degrees(marker.pad_lat as f32, marker.pad_lon as f32)
        .ok()
        .map(|coords| coords.get_point_on_sphere_ecef_km_dvec())
}
