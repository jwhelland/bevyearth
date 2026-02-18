//! Moon rendering and transform updates.

use bevy::math::DVec3;
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;

use crate::core::space::{WorldEcefKm, ecef_to_bevy_km};
use crate::orbital::MoonEcefKm;

pub const MOON_RADIUS_KM: f32 = 1737.4;
const MOON_TEXTURE_YAW_OFFSET_DEG: f32 = 0.0;

/// Marker component for the Moon entity.
#[derive(Component)]
pub struct Moon;

/// Plugin for Moon rendering.
pub struct MoonPlugin;

impl Plugin for MoonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_moon).add_systems(
            Update,
            update_moon_transform.after(crate::orbital::moon::update_moon_state),
        );
    }
}

fn spawn_moon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let mut moon_mesh: Mesh = Sphere::new(MOON_RADIUS_KM).mesh().uv(180, 90);

    // Bevy's UvSphere north pole is at local +Z.  The moon is tidal-locked via look_at so its
    // local -Z faces Earth.  Without a fix the south pole texture would face Earth.
    // We want: texture north (UvSphere +Z) → model +Y (top of face),
    //          texture prime meridian equatorial (UvSphere -X, U=0.5) → model -Z (Earth-facing).
    //   Rx(-90°): +Z → +Y  (align poles)
    //   Ry(-90°): -X → -Z  (bring near-side center to Earth-facing direction)
    let fix = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)
        * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    for attr in [Mesh::ATTRIBUTE_POSITION, Mesh::ATTRIBUTE_NORMAL] {
        if let Some(VertexAttributeValues::Float32x3(vecs)) = moon_mesh.attribute_mut(attr) {
            for v in vecs.iter_mut() {
                *v = (fix * Vec3::from(*v)).into();
            }
        }
    }

    moon_mesh.generate_tangents().unwrap();
    let mesh = meshes.add(moon_mesh);
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5),
        base_color_texture: Some(asset_server.load("moon_normal.png")),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::Visible,
        Moon,
        WorldEcefKm(DVec3::ZERO),
        Name::new("Moon"),
    ));
}

fn update_moon_transform(
    moon_pos: Res<MoonEcefKm>,
    mut query: Query<(&mut Transform, &mut Visibility, &mut WorldEcefKm), With<Moon>>,
) {
    if query.is_empty() {
        return;
    }

    for (mut transform, mut visibility, mut world_ecef) in &mut query {
        *visibility = Visibility::Visible;
        let pos_bevy = ecef_to_bevy_km(moon_pos.0);
        transform.translation = pos_bevy;

        // Tidal lock: face Earth.
        transform.look_at(Vec3::ZERO, Vec3::Y);
        transform.rotation *= Quat::from_rotation_y(MOON_TEXTURE_YAW_OFFSET_DEG.to_radians());

        world_ecef.0 = moon_pos.0;
    }
}
