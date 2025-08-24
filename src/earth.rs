use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::mesh::PrimitiveTopology;

use crate::core::coordinates::Coordinates;

pub const EARTH_RADIUS_KM: f32 = 6371.0;

/// Plugin for Earth rendering and mesh generation
pub struct EarthPlugin;

impl Plugin for EarthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_faces);
    }
}

pub fn generate_face(
    normal: Vec3,
    resolution: u32,
    x_offset: f32,
    y_offset: f32,
    // rs: &RasterData,
) -> Mesh {
    let axis_a = Vec3::new(normal.y, normal.z, normal.x); // Horizontal
    let axis_b = axis_a.cross(normal); // Vertical

    // Create a vec of verticies and indicies
    let mut verticies: Vec<Vec3> = Vec::new();
    let mut uvs = Vec::new();
    let mut indicies: Vec<u32> = Vec::new();
    let mut normals = Vec::new();
    let mut first_longitude = 0.0;
    for y in 0..(resolution) {
        for x in 0..(resolution) {
            let i = x + y * resolution;

            let percent = Vec2::new(x as f32, y as f32) / (resolution - 1) as f32;
            let point_on_unit_cube =
                normal + (percent.x - x_offset) * axis_a + (percent.y - y_offset) * axis_b;
            let point_coords: Coordinates = point_on_unit_cube.normalize().into();
            let (lat, lon) = point_coords.as_degrees();
            // Get the height value at the geographic coordinates
            // let height_offset = rs.get_coordinate_height(lat as f64, lon as f64);
            // Add the elevation to the earth_radius value of the normalized point
            // let normalized_point = if let Ok(Some(offset)) = height_offset {
            //     let height = if offset > 0.0 { offset / 1000.0 } else { 0.0 };
            //     point_on_unit_cube.normalize() * (EARTH_RADIUS + (height) as f32)
            // } else {
            let normalized_point = point_on_unit_cube.normalize() * EARTH_RADIUS_KM;
            // };

            verticies.push(normalized_point);
            let (mut u, v) = point_coords.convert_to_uv_mercator();

            if y == 0 && x == 0 {
                first_longitude = lon;
            }
            // In the middle latitudes, if we start on a negative longitude but then wind up crossing to a positive longitude, set u to 0.0 to prevent a seam
            if first_longitude < 0.0 && lon > 0.0 && lat < 89.0 && lat > -89.0 {
                u = 0.0;
            }
            // If we are below -40 degrees latitude and the tile starts at 180 degrees, set u to 0.0 to prevent a seam
            if x == 0 && lon == 180.0 && lat < -40.0 {
                u = 0.0;
            }
            uvs.push([u, v]);
            normals.push(-point_on_unit_cube.normalize());

            if x != resolution - 1 && y != resolution - 1 {
                // First triangle
                indicies.push(i);
                indicies.push(i + resolution);
                indicies.push(i + resolution + 1);

                // Second triangle
                indicies.push(i);
                indicies.push(i + resolution + 1);
                indicies.push(i + 1);
            }
        }
    }
    let indicies = Indices::U32(indicies);
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_indices(indicies);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verticies);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.generate_tangents().unwrap();
    mesh
}

pub fn generate_faces(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let faces = vec![
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::Z,
        Vec3::NEG_Z,
    ];

    let offsets = vec![(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)];
    for direction in faces {
        for offset in &offsets {
            commands
                .spawn((
                    Mesh3d(meshes.add(generate_face(direction, 100, offset.0, offset.1))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color_texture: Some(asset_server.load("world_shaded_32k.png")),
                        metallic_roughness_texture: Some(
                            asset_server.load("specular_map_inverted_8k.png"),
                        ),
                        perceptual_roughness: 1.0,
                        // normal_map_texture: Some(
                        //     asset_server.load("topography_21K.png"),
                        // ),
                        // base_color: Srgba::hex("#ffd891").unwrap().into(),
                        ..default()
                    })),
                    Transform::from_xyz(0.0, 0.0, 0.0),
                ))
                .observe(|mut trigger: Trigger<Pointer<Click>>| {
                    // Get the underlying pointer event data
                    // let click_event: &Pointer<Click> = trigger.event();
                    let hit = &trigger.event().hit;
                    if let Some(pos) = hit.position {
                        let coords: Coordinates = pos.into();
                        let (lat, lon) = coords.as_degrees();
                        info!("Latlon of selected point: Lat: {}, Lon: {}", lat, lon);
                    }
                    // Stop the event from bubbling up the entity hierarchy
                    trigger.propagate(false);
                });
        }
    }
}
