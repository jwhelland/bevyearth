use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;

use crate::core::coordinates::{Coordinates, EARTH_RADIUS_KM};

/// Plugin for Earth rendering and mesh generation
pub struct EarthPlugin;

/// Resource to store Earth mesh handle for heatmap access
#[derive(Resource)]
pub struct EarthMeshHandle {
    pub handle: Handle<Mesh>,
}

impl Plugin for EarthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_unified_earth);
    }
}

/// Generate unified Earth mesh using Bevy's UvSphere primitive
pub fn generate_unified_earth(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let mut earth_mesh: Mesh = Sphere::new(EARTH_RADIUS_KM).mesh().uv(360, 180);

    // Bevy's UvSphere places the north pole at local +Z, but this app's world convention
    // is Bevy +Y = north (ECEF.z maps to Bevy.y).  The heatmap reads mesh vertex positions
    // directly as world coordinates, so bake the fix into the mesh rather than the Transform.
    //   Rx(-90°): +Z → +Y  (poles aligned)
    //   Ry(+90°): mesh -X (U=0.5, 0° lon) → +Z  (prime meridian faces camera)
    let fix = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
        * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    for attr in [Mesh::ATTRIBUTE_POSITION, Mesh::ATTRIBUTE_NORMAL] {
        if let Some(VertexAttributeValues::Float32x3(vecs)) = earth_mesh.attribute_mut(attr) {
            for v in vecs.iter_mut() {
                *v = (fix * Vec3::from(*v)).into();
            }
        }
    }

    earth_mesh.generate_tangents().unwrap();
    let mesh_handle = meshes.add(earth_mesh);

    // Store mesh handle for heatmap access
    commands.insert_resource(EarthMeshHandle {
        handle: mesh_handle.clone(),
    });

    let material_handle = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(asset_server.load("world_shaded_32k.png")),
        metallic_roughness_texture: Some(asset_server.load("specular_map_inverted_8k.png")),
        perceptual_roughness: 1.0,
        unlit: false, // PBR lighting enabled
        ..default()
    });

    commands
        .spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Visibility::Visible,
            Name::new("Earth"),
        ))
        .observe(|mut event: On<Pointer<Click>>| {
            let hit = &event.hit;
            if let Some(pos) = hit.position {
                let coords: Coordinates = pos.into();
                let (lat, lon) = coords.as_degrees();
                info!("Latlon of selected point: Lat: {}, Lon: {}", lat, lon);
            }
            event.propagate(false);
        });
}
