//! Body-centered adapter for Bevy's first-party orbit controller.
//!
//! Only this module writes the main camera pose. Satellite positions remain ECEF
//! values; camera transforms are render-space outputs, never orbital state.
use crate::satellite::{SelectedSatellite, resources::NoradIndex};
use crate::ui::state::{CameraFocusState, CameraFocusTarget, LaunchLibraryUiState};
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy_camera_controller::pan_orbit_camera::{SyncCameraPosition, prelude::*};

mod systems;

#[derive(Component)]
pub struct MainCamera;

/// UI containers which block camera motion even over their empty background.
#[derive(Component)]
pub struct CameraInputBlocker;

#[derive(Message)]
pub enum CameraRequest {
    ToggleFocus,
}

/// The body frame used by the camera in the previous completed frame.
#[derive(Resource)]
struct BodyFrame {
    target: CameraFocusTarget,
    center: Vec3,
    rotation: Quat,
}
impl Default for BodyFrame {
    fn default() -> Self {
        Self {
            target: CameraFocusTarget::Earth,
            center: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }
}

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MinimalPanOrbitCameraPlugin)
            .init_resource::<BodyFrame>()
            .init_resource::<CameraFocusState>()
            .init_resource::<SelectedSatellite>()
            .init_resource::<NoradIndex>()
            .init_resource::<LaunchLibraryUiState>()
            .add_message::<CameraRequest>()
            .add_systems(
                PreUpdate,
                systems::camera_inputs
                    .after(PickingSystems::Last)
                    .before(SyncCameraPosition),
            )
            .add_systems(
                PostUpdate,
                systems::finalize_camera.before(TransformSystems::Propagate),
            );
    }
}

pub fn orbit_controller(distance: f32) -> PanOrbitCamera {
    let mut controller = PanOrbitCamera::default().with_initial_anchor_depth(f64::from(distance));
    controller.enabled_motion.pan = false;
    // The controller normally changes this each frame. This app uses kilometers
    // and must retain the scene's 1 km near plane, including during Moon focus.
    controller.perspective.near_clip_limits = 1.0..1.0;
    controller
}
