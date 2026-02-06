use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::collections::HashMap;

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

/// Generate unified Earth mesh using icosphere approach
pub fn generate_unified_earth(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let earth_mesh = generate_icosphere(5); // Subdivision level 5 for ~65k vertices
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

/// Generate icosphere mesh with specified subdivision levels
/// Each subdivision level quadruples the triangle count
/// Level 5 produces ~65,000 vertices (4^5 * 20 triangles * 3 vertices / triangle)
pub fn generate_icosphere(subdivisions: u32) -> Mesh {
    // Start with icosahedron vertices (12 vertices)
    let phi = (1.0 + 5.0_f32.sqrt()) / 2.0; // Golden ratio
    let vertices = vec![
        Vec3::new(-1.0, phi, 0.0).normalize(),
        Vec3::new(1.0, phi, 0.0).normalize(),
        Vec3::new(-1.0, -phi, 0.0).normalize(),
        Vec3::new(1.0, -phi, 0.0).normalize(),
        Vec3::new(0.0, -1.0, phi).normalize(),
        Vec3::new(0.0, 1.0, phi).normalize(),
        Vec3::new(0.0, -1.0, -phi).normalize(),
        Vec3::new(0.0, 1.0, -phi).normalize(),
        Vec3::new(phi, 0.0, -1.0).normalize(),
        Vec3::new(phi, 0.0, 1.0).normalize(),
        Vec3::new(-phi, 0.0, -1.0).normalize(),
        Vec3::new(-phi, 0.0, 1.0).normalize(),
    ];

    // Icosahedron faces (20 triangles)
    let mut indices = vec![
        0, 11, 5, 0, 5, 1, 0, 1, 7, 0, 7, 10, 0, 10, 11, 1, 5, 9, 5, 11, 4, 11, 10, 2, 10, 7, 6, 7,
        1, 8, 3, 9, 4, 3, 4, 2, 3, 2, 6, 3, 6, 8, 3, 8, 9, 4, 9, 5, 2, 4, 11, 6, 2, 10, 8, 6, 7, 9,
        8, 1,
    ];

    let mut vertex_positions = vertices;
    let mut vertex_cache: HashMap<(u32, u32), u32> = HashMap::new();

    // Subdivide triangles
    for _ in 0..subdivisions {
        let mut new_indices = Vec::new();
        vertex_cache.clear();

        for chunk in indices.chunks(3) {
            let v1 = chunk[0];
            let v2 = chunk[1];
            let v3 = chunk[2];

            // Get midpoint vertices (create if they don't exist)
            let a = get_midpoint_vertex(&mut vertex_positions, &mut vertex_cache, v1, v2);
            let b = get_midpoint_vertex(&mut vertex_positions, &mut vertex_cache, v2, v3);
            let c = get_midpoint_vertex(&mut vertex_positions, &mut vertex_cache, v3, v1);

            // Create 4 new triangles
            new_indices.extend_from_slice(&[v1, a, c]);
            new_indices.extend_from_slice(&[v2, b, a]);
            new_indices.extend_from_slice(&[v3, c, b]);
            new_indices.extend_from_slice(&[a, b, c]);
        }

        indices = new_indices;
    }

    // Scale vertices to Earth radius and compute UV coordinates
    let mut final_vertices = Vec::new();
    let mut uvs = Vec::new();
    let mut normals = Vec::new();

    for vertex in vertex_positions {
        let normalized = vertex.normalize();
        final_vertices.push(normalized * EARTH_RADIUS_KM);
        // Outward-facing normals for correct PBR lighting.
        normals.push(normalized);

        // Convert to geographic coordinates for UV mapping with seam handling
        let coords: Coordinates = normalized.into();
        let (u, v) = coords.convert_to_uv_mercator();
        uvs.push([u, v]);
    }

    // Fix UV seams by detecting and duplicating vertices at texture boundaries
    fix_texture_seams(&mut final_vertices, &mut uvs, &mut normals, &mut indices);

    // Create mesh
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_indices(Indices::U32(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, final_vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.generate_tangents().unwrap();

    mesh
}

/// Get or create midpoint vertex between two vertices
fn get_midpoint_vertex(
    vertices: &mut Vec<Vec3>,
    cache: &mut HashMap<(u32, u32), u32>,
    v1: u32,
    v2: u32,
) -> u32 {
    let key = if v1 < v2 { (v1, v2) } else { (v2, v1) };

    if let Some(&index) = cache.get(&key) {
        return index;
    }

    let midpoint = (vertices[v1 as usize] + vertices[v2 as usize]) / 2.0;
    let normalized = midpoint.normalize();

    vertices.push(normalized);
    let index = vertices.len() as u32 - 1;
    cache.insert(key, index);

    index
}

/// Fix texture seams by duplicating vertices that cross UV boundaries
fn fix_texture_seams(
    vertices: &mut Vec<Vec3>,
    uvs: &mut Vec<[f32; 2]>,
    normals: &mut Vec<Vec3>,
    indices: &mut Vec<u32>,
) {
    let mut new_vertices = vertices.clone();
    let mut new_uvs = uvs.clone();
    let mut new_normals = normals.clone();
    let mut new_indices = Vec::new();

    // Process triangles to fix UV seams
    for triangle in indices.chunks(3) {
        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;

        let uv0 = uvs[i0];
        let uv1 = uvs[i1];
        let uv2 = uvs[i2];

        // Check for large UV jumps that indicate crossing the 180° meridian
        let du01 = (uv0[0] - uv1[0]).abs();
        let du02 = (uv0[0] - uv2[0]).abs();
        let du12 = (uv1[0] - uv2[0]).abs();

        let max_du = du01.max(du02).max(du12);

        if max_du > 0.5 {
            // This triangle crosses the seam, need to fix UVs
            let mut fixed_indices = [triangle[0], triangle[1], triangle[2]];

            // Duplicate vertices and adjust UV coordinates to prevent wrap-around
            for j in 0..3 {
                let idx = triangle[j] as usize;
                let u = uvs[idx][0];

                // If this vertex is on the "wrong" side of the seam for this triangle
                if u < 0.25 && max_du > 0.5 {
                    // Duplicate vertex with adjusted UV
                    new_vertices.push(vertices[idx]);
                    new_normals.push(normals[idx]);
                    new_uvs.push([u + 1.0, uvs[idx][1]]);
                    fixed_indices[j] = new_vertices.len() as u32 - 1;
                }
            }

            new_indices.extend_from_slice(&fixed_indices);
        } else {
            // Normal triangle, no seam crossing
            new_indices.extend_from_slice(triangle);
        }
    }

    // Update the arrays with fixed data
    *vertices = new_vertices;
    *uvs = new_uvs;
    *normals = new_normals;
    *indices = new_indices;
}
