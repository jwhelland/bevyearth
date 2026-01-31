//! BigSpace integration helpers and shared configuration.

use bevy::math::DVec3;
use bevy::prelude::*;
use big_space::commands::BigSpaceCommands;
use big_space::prelude::{CellCoord, Grid};

use crate::core::space::ecef_to_bevy_km_dvec;

pub const BIG_SPACE_CELL_EDGE_KM: f32 = 100_000.0;
pub const BIG_SPACE_SWITCH_THRESHOLD_KM: f32 = 50_000.0;

/// Startup ordering so BigSpace is ready before scene spawn.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum StartupSet {
    BigSpace,
    Scene,
}

/// Resource pointing at the root BigSpace entity.
#[derive(Resource, Copy, Clone, Debug)]
pub struct BigSpaceRoot(pub Entity);

/// Spawn the BigSpace root/grid and stash it as a resource.
pub fn setup_big_space_root(mut commands: Commands) {
    let grid = Grid::new(BIG_SPACE_CELL_EDGE_KM, BIG_SPACE_SWITCH_THRESHOLD_KM);
    let mut root_entity = Entity::PLACEHOLDER;
    commands.spawn_big_space(grid, |root| {
        root_entity = root.id();
    });
    // Ensure visibility propagation is consistent for children that rely on inherited visibility.
    // Without this, Bevy will warn (B0004) and culling can behave erratically.
    commands.entity(root_entity).insert((
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));
    commands.insert_resource(BigSpaceRoot(root_entity));
}

/// Convert a Bevy-space absolute translation (km, f64) into a BigSpace cell + local translation.
pub fn bevy_abs_to_cell_local(grid: &Grid, bevy_abs_km: DVec3) -> (CellCoord, Vec3) {
    grid.translation_to_grid(bevy_abs_km)
}

/// Convert an ECEF absolute translation (km, f64) into a BigSpace cell + local translation.
pub fn ecef_to_cell_local(grid: &Grid, ecef_km: DVec3) -> (CellCoord, Vec3) {
    let bevy_abs_km = ecef_to_bevy_km_dvec(ecef_km);
    grid.translation_to_grid(bevy_abs_km)
}

/// Convert a BigSpace cell + local translation into render-space coordinates using a floating origin.
pub fn cell_local_to_render(
    grid: &Grid,
    cell: CellCoord,
    local: Vec3,
    origin_cell: CellCoord,
    origin_local: Vec3,
) -> Vec3 {
    let cell_world = cell.as_dvec3(grid);
    let origin_world = origin_cell.as_dvec3(grid);
    let local_world = DVec3::from(local);
    let origin_local_world = DVec3::from(origin_local);
    let relative = (cell_world + local_world) - (origin_world + origin_local_world);
    relative.as_vec3()
}

/// Convert an ECEF absolute translation to render-space coordinates using a floating origin.
pub fn ecef_to_render(
    grid: &Grid,
    ecef_km: DVec3,
    origin_cell: CellCoord,
    origin_local: Vec3,
) -> Vec3 {
    let (cell, local) = ecef_to_cell_local(grid, ecef_km);
    cell_local_to_render(grid, cell, local, origin_cell, origin_local)
}

/// Get the current render origin for a grid (cell + local), driven by BigSpace.
pub fn render_origin_from_grid(grid: &Grid) -> (CellCoord, Vec3) {
    let origin = grid.local_floating_origin();
    (origin.cell(), origin.translation())
}
