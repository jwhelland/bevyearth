//! Satellite visibility heatmap implementation
//!
//! This module provides real-time satellite visibility heatmapping on the Earth surface.
//! It colors Earth mesh vertices based on the number of visible satellites from each point,
//! using efficient chunked updates for smooth performance.

use bevy::ecs::system::SystemParam;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::tasks::{ComputeTaskPool, Task, block_on};
use std::time::Instant;

use crate::core::coordinates::{
    EARTH_RADIUS_KM, hemisphere_prefilter_ecef_dvec, los_visible_ecef_dvec,
};
use crate::core::space::{WorldEcefKm, bevy_to_ecef_km};
use crate::satellite::Satellite;
use crate::visualization::colormaps::turbo_colormap;
use crate::visualization::earth::EarthMeshHandle;

/// Component to mark the heatmap overlay entity
#[derive(Component)]
struct HeatmapOverlay;

/// Configuration resource for heatmap behavior
#[derive(Resource, Clone, Debug)]
pub struct HeatmapConfig {
    /// Enable/disable heatmap rendering
    pub enabled: bool,
    /// Update period in seconds (0.5 recommended for smooth updates)
    pub update_period_s: f32,
    /// Alpha transparency for heatmap colors (0.0-1.0)
    pub color_alpha: f32,
    /// Range normalization mode
    pub range_mode: RangeMode,
    /// Fixed maximum count for normalization (used when range_mode is Fixed)
    pub fixed_max: Option<u32>,
    /// Performance tuning: vertices to process per frame
    pub chunk_size: usize,
    /// Performance tuning: chunks to process per frame
    pub chunks_per_frame: usize,
}

/// Range normalization modes for color mapping
#[derive(Clone, Debug, PartialEq)]
pub enum RangeMode {
    /// Auto-scale based on current min/max values
    Auto,
    /// Use fixed maximum value
    Fixed,
}

impl Default for HeatmapConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            update_period_s: 0.5,
            color_alpha: 0.7,
            range_mode: RangeMode::Auto,
            fixed_max: Some(20),
            chunk_size: 2000,
            chunks_per_frame: 1,
        }
    }
}

/// Runtime state for heatmap system
#[derive(Resource)]
pub struct HeatmapState {
    /// Last update timestamp
    pub last_update_instant: Instant,
    /// Earth mesh handle for vertex color updates
    pub earth_mesh_handle: Option<Handle<Mesh>>,
    /// Visibility counts per vertex
    pub vertex_counts: Vec<u32>,
    /// Computed color buffer for vertices
    pub color_buffer: Vec<[f32; 4]>,
    /// Vertex positions (cached for performance)
    pub vertex_positions: Vec<Vec3>,
    /// Whether vertex positions have been cached
    pub positions_cached: bool,
    /// In-flight async task for computing visibility counts
    pub pending_task: Option<Task<Vec<u32>>>,
}

#[derive(SystemParam)]
struct HeatmapParams<'w, 's> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    satellite_query: Query<'w, 's, &'static WorldEcefKm, With<Satellite>>,
    heatmap_query: Query<
        'w,
        's,
        (&'static Mesh3d, &'static MeshMaterial3d<StandardMaterial>),
        With<HeatmapOverlay>,
    >,
}

impl Default for HeatmapState {
    fn default() -> Self {
        Self {
            last_update_instant: Instant::now(),
            earth_mesh_handle: None,
            vertex_counts: Vec::new(),
            color_buffer: Vec::new(),
            vertex_positions: Vec::new(),
            positions_cached: false,
            pending_task: None,
        }
    }
}

/// Plugin for satellite visibility heatmap
pub struct HeatmapPlugin;

impl Plugin for HeatmapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeatmapConfig>()
            .init_resource::<HeatmapState>()
            .add_systems(
                Update,
                (
                    initialize_heatmap_system,
                    update_heatmap_system,
                    toggle_heatmap_visibility,
                )
                    .chain(),
            );
    }
}

/// Initialize heatmap system when Earth mesh handle becomes available
fn initialize_heatmap_system(
    earth_mesh_handle: Option<Res<EarthMeshHandle>>,
    mut state: ResMut<HeatmapState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Some(handle_res) = earth_mesh_handle
        && state.earth_mesh_handle.is_none()
    {
        state.earth_mesh_handle = Some(handle_res.handle.clone());

        // Initialize vertex buffers based on mesh
        if let Some(mesh) = meshes.get(&handle_res.handle)
            && let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            let vertex_count = positions.len();
            state.vertex_counts.resize(vertex_count, 0);
            state
                .color_buffer
                .resize(vertex_count, [0.0, 0.0, 0.0, 0.0]);

            // Create a separate heatmap overlay entity with its own mesh copy
            let overlay_mesh_handle = create_heatmap_overlay(
                &mut commands,
                &mut materials,
                &mut meshes,
                &handle_res.handle,
            );
            state.earth_mesh_handle = Some(overlay_mesh_handle);
        }
    }
}

/// Create a separate heatmap overlay entity with its own mesh copy
fn create_heatmap_overlay(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    original_mesh_handle: &Handle<Mesh>,
) -> Handle<Mesh> {
    // Clone the original mesh to create a separate mesh for the heatmap
    let overlay_mesh = if let Some(original_mesh) = meshes.get(original_mesh_handle) {
        let mut cloned_mesh = original_mesh.clone();

        // Initialize with transparent vertex colors
        if let Some(positions) = cloned_mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            let vertex_count = positions.len();
            let transparent_colors: Vec<[f32; 4]> = vec![[0.0, 0.0, 0.0, 0.0]; vertex_count];
            cloned_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, transparent_colors);
        }

        cloned_mesh
    } else {
        // Fallback if original mesh not found
        return original_mesh_handle.clone();
    };

    let overlay_mesh_handle = meshes.add(overlay_mesh);

    // Create a transparent material that will show vertex colors
    let heatmap_material = materials.add(StandardMaterial {
        base_color: Color::WHITE.with_alpha(0.0), // Start completely transparent
        alpha_mode: AlphaMode::Blend,
        unlit: true, // No lighting calculations, pure vertex colors
        ..default()
    });

    commands.spawn((
        Mesh3d(overlay_mesh_handle.clone()),
        MeshMaterial3d(heatmap_material),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(1.001)), // Slightly larger to sit on top
        // Keep this from affecting the scene when heatmap is disabled.
        Visibility::Hidden,
        HeatmapOverlay, // Mark this entity as the heatmap overlay
    ));

    overlay_mesh_handle
}

/// Main heatmap update system with chunked processing
fn update_heatmap_system(
    config: Res<HeatmapConfig>,
    mut state: ResMut<HeatmapState>,
    params: HeatmapParams,
) {
    let HeatmapParams {
        mut meshes,
        mut materials,
        satellite_query,
        heatmap_query,
    } = params;

    if !config.enabled {
        return;
    }

    if state.earth_mesh_handle.is_none() {
        return;
    }

    // Get the heatmap overlay entity
    let Ok((mesh3d, material3d)) = heatmap_query.single() else {
        warn!("Heatmap overlay entity not found!");
        return;
    };

    let mesh = match meshes.get_mut(&mesh3d.0) {
        Some(mesh) => mesh,
        None => return,
    };

    // Cache vertex positions on first run
    if !state.positions_cached
        && let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        state.vertex_positions = positions.iter().map(|&pos| Vec3::from(pos)).collect();
        state.positions_cached = true;
    }

    if state.vertex_positions.is_empty() {
        return;
    }

    // Collect current satellite positions in ECEF
    let satellite_positions_ecef: Vec<DVec3> = collect_satellite_positions_ecef(&satellite_query);

    if satellite_positions_ecef.is_empty() {
        // No satellites - completely hide the heatmap overlay
        if let Some(material) = materials.get_mut(&material3d.0) {
            material.base_color.set_alpha(0.0);
        }

        // Clear vertex colors so they don't interfere
        clear_vertex_colors(mesh);

        state.last_update_instant = Instant::now();
        return;
    }

    // Spawn new async task if ready
    if state.pending_task.is_none()
        && state.last_update_instant.elapsed().as_secs_f32() >= config.update_period_s
    {
        let positions = state.vertex_positions.clone();
        let satellites = satellite_positions_ecef.clone();
        let earth_radius = EARTH_RADIUS_KM as f64;

        let task = ComputeTaskPool::get().spawn(async move {
            let mut counts = vec![0u32; positions.len()];
            for (i, vertex_pos) in positions.iter().enumerate() {
                let surface_point_bevy = vertex_pos.normalize() * EARTH_RADIUS_KM;
                let surface_point_ecef = bevy_to_ecef_km(surface_point_bevy);
                counts[i] =
                    count_visible_satellites(&surface_point_ecef, &satellites, earth_radius);
            }
            counts
        });

        state.pending_task = Some(task);
        state.last_update_instant = Instant::now();
    }

    // Poll task completion
    if let Some(task) = state.pending_task.take() {
        if task.is_finished() {
            let counts = block_on(task);
            state.vertex_counts = counts;
            let counts = state.vertex_counts.clone();

            apply_colors_to_mesh(mesh, &counts, &config, state.color_buffer.as_mut_slice());

            if let Some(material) = materials.get_mut(&material3d.0) {
                material.base_color.set_alpha(1.0);
            }
        } else {
            state.pending_task = Some(task);
        }
    }
}

/// Collect satellite positions in ECEF coordinates
fn collect_satellite_positions_ecef(
    satellite_query: &Query<&WorldEcefKm, With<Satellite>>,
) -> Vec<DVec3> {
    satellite_query
        .iter()
        .map(|world_ecef| world_ecef.0)
        .collect()
}

/// Count visible satellites from a given surface point
fn count_visible_satellites(
    surface_point: &DVec3,
    satellite_positions: &[DVec3],
    earth_radius_km: f64,
) -> u32 {
    let mut visible_count = 0;

    // Check visibility for each satellite
    for &sat_pos in satellite_positions {
        // Pre-filter using hemisphere check
        if hemisphere_prefilter_ecef_dvec(*surface_point, sat_pos, earth_radius_km) {
            // Check line-of-sight visibility
            if los_visible_ecef_dvec(*surface_point, sat_pos, earth_radius_km) {
                visible_count += 1;
            }
        }
    }

    visible_count
}

/// Apply computed colors to mesh vertex colors
fn apply_colors_to_mesh(
    mesh: &mut Mesh,
    vertex_counts: &[u32],
    config: &HeatmapConfig,
    color_buffer: &mut [[f32; 4]],
) {
    if vertex_counts.is_empty() {
        return;
    }

    // Determine normalization range
    let (min_count, max_count) = match config.range_mode {
        RangeMode::Auto => {
            let min = *vertex_counts.iter().min().unwrap_or(&0);
            let max = *vertex_counts.iter().max().unwrap_or(&1);
            (min, max.max(1)) // Ensure max is at least 1 to avoid division by zero
        }
        RangeMode::Fixed => (0, config.fixed_max.unwrap_or(20)),
    };

    // Map counts to colors
    for (i, &count) in vertex_counts.iter().enumerate() {
        if count == 0 {
            // Zero count should be transparent
            color_buffer[i] = [0.0, 0.0, 0.0, 0.0];
        } else {
            let normalized = if max_count > min_count {
                (count - min_count) as f32 / (max_count - min_count) as f32
            } else {
                0.0
            };

            let mut color = turbo_colormap(normalized.clamp(0.0, 1.0));
            color[3] = config.color_alpha; // Apply alpha
            color_buffer[i] = color;
        }
    }

    // Apply colors to mesh in place when possible
    if let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
    {
        for (dst, src) in colors.iter_mut().zip(color_buffer.iter()) {
            *dst = *src;
        }
    } else {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, color_buffer.to_vec());
    }
}

/// Toggle heatmap overlay visibility based on config
fn toggle_heatmap_visibility(
    config: Res<HeatmapConfig>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut heatmap_query: Query<
        (&Mesh3d, &MeshMaterial3d<StandardMaterial>, &mut Visibility),
        With<HeatmapOverlay>,
    >,
) {
    if config.is_changed()
        && let Ok((mesh3d, material3d, mut visibility)) = heatmap_query.single_mut()
        && let Some(material) = materials.get_mut(&material3d.0)
    {
        if config.enabled {
            *visibility = Visibility::Visible;
            // Enable heatmap - make material visible
            material.base_color.set_alpha(1.0);
        } else {
            *visibility = Visibility::Hidden;
            // Disable heatmap - hide completely
            material.base_color.set_alpha(0.0);

            // Also clear vertex colors to prevent lingering effects
            if let Some(mesh) = meshes.get_mut(&mesh3d.0) {
                clear_vertex_colors(mesh);
            }
        }
    }
}

/// Clear all vertex colors from a mesh (set to transparent)
fn clear_vertex_colors(mesh: &mut Mesh) {
    if let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        let vertex_count = positions.len();
        let transparent_colors: Vec<[f32; 4]> = vec![[0.0, 0.0, 0.0, 0.0]; vertex_count];
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, transparent_colors);
    }
}
