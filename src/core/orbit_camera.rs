//! Glue to make `bevy_panorbit_camera` work with `big_space` floating origin.
//!
//! Key idea: keep the actual camera as a low-precision entity (no `CellCoord`) so PanOrbit can
//! freely write `Transform.translation`. A separate high-precision entity (with `CellCoord`) is
//! marked as the `FloatingOrigin` and is recentered to keep the camera's local translation small.

use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraSystemSet};
use big_space::prelude::{CellCoord, FloatingOrigin, Grid};

use crate::core::big_space::BigSpaceRoot;

/// Marks the entity that acts as the high-precision floating origin for a PanOrbit camera.
#[derive(Component)]
pub struct PanOrbitFloatingOrigin;

pub struct BigSpacePanOrbitPlugin;

impl Plugin for BigSpacePanOrbitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            recenter_panorbit_origin
                .after(PanOrbitCameraSystemSet)
                // Ensure camera matrices (and anything else that reads the camera transform)
                // observe the recentered state for this frame.
                .before(bevy::camera::CameraUpdateSystems)
                .before(TransformSystems::Propagate),
        );
    }
}

fn recenter_panorbit_origin(
    big_space_root: Res<BigSpaceRoot>,
    grid_query: Query<&Grid>,
    mut origin_query: Query<
        (&mut CellCoord, &Children),
        (With<FloatingOrigin>, With<PanOrbitFloatingOrigin>),
    >,
    mut camera_query: Query<(&mut Transform, &mut PanOrbitCamera), With<Camera3d>>,
) {
    let Ok(grid) = grid_query.get(big_space_root.0) else {
        warn!("recenter_panorbit_origin: BigSpace grid not found");
        return;
    };
    let Ok((mut origin_cell, children)) = origin_query.single_mut() else {
        warn!("recenter_panorbit_origin: floating origin entity not found");
        return;
    };

    let mut cam_entity = None;
    for e in children.iter() {
        if camera_query.contains(e) {
            cam_entity = Some(e);
            break;
        }
    }
    let Some(cam_entity) = cam_entity else {
        warn!("recenter_panorbit_origin: camera not found among origin children");
        return;
    };
    let Ok((mut cam_transform, mut pan_orbit)) = camera_query.get_mut(cam_entity) else {
        warn!("recenter_panorbit_origin: failed to query camera entity");
        return;
    };

    // Recenter only when the local translation is "too large", matching big_space's own
    // hysteresis via `maximum_distance_from_origin`. This avoids cell thrashing near boundaries,
    // which can look like random teleportation/flicker while zooming.
    if cam_transform.translation.abs().max_element() <= grid.maximum_distance_from_origin() {
        return;
    }

    let (cell_delta, new_translation) =
        grid.imprecise_translation_to_grid(cam_transform.translation);
    if cell_delta == CellCoord::ZERO {
        return;
    }

    let delta_d = cell_delta.as_dvec3(grid);
    let delta = Vec3::new(delta_d.x as f32, delta_d.y as f32, delta_d.z as f32);

    // Shift the origin cell and keep PanOrbit's notion of focus stable.
    // All three focus fields must be updated together to prevent jumps:
    // - focus: current focus position
    // - target_focus: animated target for smooth transitions
    // - focus_bounds_origin: reference point for focus bounds clamping
    *origin_cell += cell_delta;
    pan_orbit.focus -= delta;
    pan_orbit.target_focus -= delta;
    pan_orbit.focus_bounds_origin -= delta;

    // Keep the camera abs position identical, but ensure local translation is small.
    cam_transform.translation = new_translation;
}
