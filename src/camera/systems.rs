//! Body-centered adapter for Bevy's first-party orbit controller.
//!
//! Only this module writes the main camera pose. Satellite positions remain ECEF
//! values; camera transforms are render-space outputs, never orbital state.
use bevy::camera::RenderTarget;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
#[cfg(test)]
use bevy::math::DVec3;
use bevy::picking::{
    hover::HoverMap,
    pointer::{PointerAction, PointerId, PointerInput, PointerLocation},
};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy::window::PrimaryWindow;
use bevy_camera_controller::pan_orbit_camera::controller::motion::CurrentMotion;
use bevy_camera_controller::pan_orbit_camera::prelude::*;

use crate::core::{
    coordinates::Coordinates,
    space::{WorldEcefKm, ecef_to_bevy_km},
};
use crate::launch_library::LaunchLibraryData;
use crate::orbital::{Dut1, MoonEcefKm, SimulationTime, moon_position_ecef_km};
use crate::satellite::{SelectedSatellite, components::Satellite, resources::NoradIndex};
use crate::ui::state::{
    CameraFocusState, CameraFocusTarget, CameraPose, LaunchLibraryItemKind, LaunchLibrarySelection,
    LaunchLibraryUiState, UiLayoutState,
};
use crate::visualization::moon::Moon;

use super::{BodyFrame, CameraInputBlocker, CameraRequest, MainCamera};

fn cancel_motion(controller: &mut PanOrbitCamera) {
    // end_move() deliberately creates momentum; a programmatic move needs none.
    controller.current_motion = CurrentMotion::Stationary;
}

fn input_allowed(active: bool, over_ui: bool, resizing: bool, automatic: bool) -> bool {
    active && !over_ui && !resizing && !automatic
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn camera_inputs(
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheels: MessageReader<MouseWheel>,
    mut moves: MessageReader<PointerInput>,
    pointers: Query<(&PointerId, &PointerLocation)>,
    primary: Query<Entity, With<PrimaryWindow>>,
    window: Query<&Window, With<PrimaryWindow>>,
    hover: Res<HoverMap>,
    ui: Query<(), With<Node>>,
    blockers: Query<(&Node, &ComputedNode, &UiGlobalTransform), With<CameraInputBlocker>>,
    layout: Option<Res<UiLayoutState>>,
    selected: Res<SelectedSatellite>,
    launch: Res<LaunchLibraryUiState>,
    frame: Res<BodyFrame>,
    mut cameras: Query<(&Camera, &RenderTarget, &Transform, &mut PanOrbitCamera), With<MainCamera>>,
) {
    // Drain our readers even when the UI owns input; never replay old wheel data.
    let zoom = wheels
        .read()
        .map(|w| {
            w.y * match w.unit {
                MouseScrollUnit::Line => 150.0,
                MouseScrollUnit::Pixel => 1.0,
            }
        })
        .sum::<f32>();
    let delta = moves
        .read()
        .filter(|m| m.pointer_id == PointerId::Mouse)
        .filter_map(|m| match m.action {
            PointerAction::Move { delta } => Some(delta),
            _ => None,
        })
        .sum::<Vec2>();
    let location = pointers
        .iter()
        .find(|(id, _)| **id == PointerId::Mouse)
        .and_then(|(_, p)| p.location());
    let physical = window
        .single()
        .ok()
        .and_then(Window::physical_cursor_position);
    let over_ui = hover
        .get(&PointerId::Mouse)
        .is_some_and(|hits| hits.keys().any(|e| ui.contains(*e)))
        || physical.is_some_and(|p| {
            blockers.iter().any(|(node, computed, transform)| {
                node.display != Display::None
                    && computed.size().min_element() > 0.0
                    && computed.contains_point(*transform, p)
            })
        });
    let resizing = layout.is_some_and(|l| l.resizing_left_panel || l.resizing_right_panel);
    let automatic = selected.tracking.is_some()
        || selected.selected.is_some()
        || launch.camera_target.is_some();
    for (camera, target, transform, mut controller) in &mut cameras {
        let in_view = location.is_some_and(|p| p.is_in_viewport(camera, target, &primary));
        if !input_allowed(camera.is_active && in_view, over_ui, resizing, automatic) {
            cancel_motion(&mut controller);
            continue;
        }
        let anchor = transform
            .compute_affine()
            .inverse()
            .transform_point3(frame.center)
            .as_dvec3();
        if mouse.just_pressed(MouseButton::Left) {
            controller.start_orbit(Some(anchor));
        }
        if mouse.just_released(MouseButton::Left) {
            controller.end_move();
        }
        if zoom != 0.0 && !mouse.pressed(MouseButton::Left) {
            controller.start_zoom(Some(anchor));
        }
        if mouse.pressed(MouseButton::Left) {
            controller.send_screenspace_input(delta);
        }
        controller.send_zoom_input(zoom);
        if zoom == 0.0
            && controller.current_motion.is_zooming_only()
            && controller
                .current_motion
                .inputs()
                .is_none_or(|i| i.zoom_velocity_abs(controller.smoothing.zoom.mul_f32(2.0)) <= 0.0)
        {
            controller.end_move();
        }
    }
}

pub fn pose_from_camera(position: Vec3, center: Vec3) -> CameraPose {
    let delta = position - center;
    let radius = delta.length().max(1.0);
    let direction = delta / radius;
    CameraPose {
        radius,
        yaw: direction.x.atan2(direction.z),
        pitch: direction.y.clamp(-1.0, 1.0).asin(),
    }
}

fn position_from_pose(center: Vec3, pose: CameraPose) -> Vec3 {
    center
        + Vec3::new(
            pose.pitch.cos() * pose.yaw.sin(),
            pose.pitch.sin(),
            pose.pitch.cos() * pose.yaw.cos(),
        ) * pose.radius
}

fn apply_pose(
    controller: &mut PanOrbitCamera,
    transform: &mut Transform,
    center: Vec3,
    pose: CameraPose,
) {
    cancel_motion(controller);
    transform.translation = position_from_pose(center, pose);
    transform.look_at(center, Vec3::Y);
    controller.last_anchor_depth = -f64::from(pose.radius);
}

fn yaw_delta(from: f32, to: f32) -> f32 {
    (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

fn interpolate_pose(from: CameraPose, to: CameraPose, smooth: f32, dt: f32) -> CameraPose {
    let t = 1.0 - (1.0 - smooth.clamp(0.0, 1.0)).powf(dt.max(0.0) * 60.0);
    CameraPose {
        radius: from.radius + (to.radius - from.radius) * t,
        yaw: from.yaw + yaw_delta(from.yaw, to.yaw) * t,
        pitch: from.pitch + (to.pitch - from.pitch) * t,
    }
}

fn transport_position(position: Vec3, old: &BodyFrame, center: Vec3, rotation: Quat) -> Vec3 {
    center + rotation * old.rotation.inverse() * (position - old.center)
}

fn satellite_pose(
    norad: u32,
    index: &NoradIndex,
    positions: &Query<&WorldEcefKm, With<Satellite>>,
    offset: f32,
) -> Option<CameraPose> {
    let position = ecef_to_bevy_km(positions.get(*index.map.get(&norad)?).ok()?.0);
    if !position.is_finite() || position.length_squared() < 1.0 {
        return None;
    }
    let mut pose = pose_from_camera(position, Vec3::ZERO);
    pose.radius += offset;
    Some(pose)
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn finalize_camera(
    mut requests: MessageReader<CameraRequest>,
    mut frame: ResMut<BodyFrame>,
    mut focus: ResMut<CameraFocusState>,
    mut selected: ResMut<SelectedSatellite>,
    index: Res<NoradIndex>,
    positions: Query<&WorldEcefKm, With<Satellite>>,
    moon_pos: Option<Res<MoonEcefKm>>,
    sim: Option<Res<SimulationTime>>,
    dut1: Option<Res<Dut1>>,
    moons: Query<&Transform, (With<Moon>, Without<MainCamera>)>,
    data: Option<Res<LaunchLibraryData>>,
    mut launch: ResMut<LaunchLibraryUiState>,
    mut last_launch: Local<Option<LaunchLibrarySelection>>,
    mut last_tracking: Local<Option<u32>>,
    time: Res<Time>,
    mut cameras: Query<(&mut PanOrbitCamera, &mut Transform), With<MainCamera>>,
) {
    let Ok((mut controller, mut transform)) = cameras.single_mut() else {
        requests.clear();
        return;
    };
    let moon_center = moon_pos
        .as_ref()
        .map(|p| ecef_to_bevy_km(p.0))
        .filter(|p| p.length_squared() > 0.0)
        .or_else(|| {
            sim.as_ref()
                .zip(dut1.as_ref())
                .map(|(s, d)| ecef_to_bevy_km(moon_position_ecef_km(s.current_utc, ***d)))
        })
        .unwrap_or(Vec3::ZERO);
    let moon_rotation = moons.single().map_or(Quat::IDENTITY, |t| t.rotation);
    if focus.target == CameraFocusTarget::Moon && frame.target == CameraFocusTarget::Moon {
        // The controller has already applied user input in the previous body
        // frame. Transport that result to the new body frame, retaining its
        // local surface direction as well as any new user orbit/zoom.
        transform.translation =
            transport_position(transform.translation, &frame, moon_center, moon_rotation);
    }
    for request in requests.read() {
        match request {
            CameraRequest::ToggleFocus => {
                selected.tracking = None;
                selected.selected = None;
                launch.camera_target = None;
                match focus.target {
                    CameraFocusTarget::Earth => {
                        focus.last_earth_pose =
                            Some(pose_from_camera(transform.translation, Vec3::ZERO));
                        focus.target = CameraFocusTarget::Moon;
                        let direction = moon_center.try_normalize().unwrap_or(Vec3::Z);
                        let pose = pose_from_camera(direction * 10_000.0, Vec3::ZERO);
                        apply_pose(&mut controller, &mut transform, moon_center, pose);
                    }
                    CameraFocusTarget::Moon => {
                        focus.target = CameraFocusTarget::Earth;
                        let pose = focus.last_earth_pose.unwrap_or(CameraPose {
                            radius: 25_000.0,
                            yaw: 0.0,
                            pitch: 0.0,
                        });
                        apply_pose(&mut controller, &mut transform, Vec3::ZERO, pose);
                    }
                }
            }
        }
    }
    // A popup's text/animation updates must not restart its fly-to every frame.
    if launch.selection != *last_launch {
        *last_launch = launch.selection;
        if let Some(selection) = launch
            .selection
            .filter(|s| s.kind == LaunchLibraryItemKind::Launch)
            && let Some(item) = data.as_ref().and_then(|d| d.launches.get(selection.index))
            && let (Some(lat), Some(lon)) = (item.pad_lat, item.pad_lon)
            && let Ok(coords) = Coordinates::from_degrees(lat as f32, lon as f32)
        {
            let center = if focus.target == CameraFocusTarget::Moon {
                moon_center
            } else {
                Vec3::ZERO
            };
            let radius = pose_from_camera(transform.translation, center)
                .radius
                .min(9_000.0);
            let mut pose = pose_from_camera(
                ecef_to_bevy_km(coords.get_point_on_sphere_ecef_km_dvec()),
                Vec3::ZERO,
            );
            pose.radius = radius;
            focus.target = CameraFocusTarget::Earth;
            selected.tracking = None;
            selected.selected = None;
            launch.camera_target = Some(pose);
            cancel_motion(&mut controller);
        }
    }
    let new_tracking = selected.tracking.is_some() && selected.tracking != *last_tracking;
    if new_tracking || selected.selected.is_some() {
        focus.target = CameraFocusTarget::Earth;
        launch.camera_target = None;
        cancel_motion(&mut controller);
    }
    if let Some(norad) = selected.selected.take()
        && let Some(pose) = satellite_pose(norad, &index, &positions, 5_000.0)
    {
        apply_pose(&mut controller, &mut transform, Vec3::ZERO, pose);
    }
    if let Some(norad) = selected.tracking {
        if let Some(target) = satellite_pose(norad, &index, &positions, selected.tracking_offset) {
            let current = pose_from_camera(transform.translation, Vec3::ZERO);
            let pose = interpolate_pose(current, target, selected.smooth_factor, time.delta_secs());
            apply_pose(&mut controller, &mut transform, Vec3::ZERO, pose);
        } else {
            selected.tracking = None;
            cancel_motion(&mut controller);
        }
    }
    *last_tracking = selected.tracking;
    if let Some(target) = launch.camera_target {
        let current = pose_from_camera(transform.translation, Vec3::ZERO);
        let pose = interpolate_pose(current, target, 0.12, time.delta_secs());
        apply_pose(&mut controller, &mut transform, Vec3::ZERO, pose);
        if (pose.radius - target.radius).abs() < 5.0
            && (pose.pitch - target.pitch).abs() < 0.002
            && yaw_delta(pose.yaw, target.yaw).abs() < 0.002
        {
            launch.camera_target = None;
        }
    }
    let center = if focus.target == CameraFocusTarget::Moon {
        moon_center
    } else {
        Vec3::ZERO
    };
    transform.look_at(center, Vec3::Y);
    controller.last_anchor_depth = -f64::from(transform.translation.distance(center).max(1.0));
    *frame = BodyFrame {
        target: focus.target,
        center,
        rotation: if focus.target == CameraFocusTarget::Moon {
            moon_rotation
        } else {
            Quat::IDENTITY
        },
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::orbit_controller;

    #[test]
    fn pose_round_trip_at_earth_and_moon() {
        for center in [Vec3::ZERO, Vec3::new(380_000.0, 12_000.0, -40_000.0)] {
            for yaw in [-3.0, -0.1, 2.7] {
                let pose = CameraPose {
                    radius: 25_000.0,
                    yaw,
                    pitch: 0.4,
                };
                let actual = pose_from_camera(position_from_pose(center, pose), center);
                assert!((actual.radius - pose.radius).abs() < 0.1);
                assert!(yaw_delta(actual.yaw, pose.yaw).abs() < 0.00001);
                assert!((actual.pitch - pose.pitch).abs() < 0.00001);
            }
        }
    }

    #[test]
    fn smoothing_uses_shortest_yaw_path_and_elapsed_time() {
        let a = CameraPose {
            radius: 10_000.0,
            yaw: 3.13,
            pitch: 0.0,
        };
        let b = CameraPose {
            radius: 20_000.0,
            yaw: -3.13,
            pitch: 0.4,
        };
        let half = interpolate_pose(a, b, 0.1, 1.0 / 120.0);
        let twice = interpolate_pose(half, b, 0.1, 1.0 / 120.0);
        let once = interpolate_pose(a, b, 0.1, 1.0 / 60.0);
        assert!((twice.radius - once.radius).abs() < 0.01);
        assert!(yaw_delta(a.yaw, once.yaw).abs() < 0.01);
        assert!(yaw_delta(twice.yaw, once.yaw).abs() < 0.00001);
    }

    #[test]
    fn moon_transport_preserves_surface_direction_and_user_zoom() {
        let old = BodyFrame {
            target: CameraFocusTarget::Moon,
            center: Vec3::new(380_000.0, 0.0, 0.0),
            rotation: Quat::from_rotation_y(0.3),
        };
        let center = Vec3::new(390_000.0, 10_000.0, 0.0);
        let rotation = Quat::from_rotation_y(0.8);
        let local = Vec3::new(1.0, 0.3, -0.2).normalize() * 8_000.0;
        let position = old.center + old.rotation * local;
        let actual =
            rotation.inverse() * (transport_position(position, &old, center, rotation) - center);
        assert!(actual.distance(local) < 0.1);
    }

    #[test]
    fn ui_and_automatic_motion_block_camera_input() {
        assert!(input_allowed(true, false, false, false));
        assert!(!input_allowed(false, false, false, false));
        assert!(!input_allowed(true, true, false, false));
        assert!(!input_allowed(true, false, true, false));
        assert!(!input_allowed(true, false, false, true));
    }

    #[test]
    fn programmatic_pose_clears_user_motion_and_updates_anchor() {
        let mut controller = orbit_controller(25_000.0);
        controller.start_orbit(Some(DVec3::new(0.0, 0.0, -25_000.0)));
        controller.send_screenspace_input(Vec2::new(100.0, 0.0));
        controller.end_move();
        let mut transform = Transform::default();
        apply_pose(
            &mut controller,
            &mut transform,
            Vec3::ZERO,
            CameraPose {
                radius: 9_000.0,
                yaw: 1.0,
                pitch: 0.2,
            },
        );
        assert!(matches!(
            controller.current_motion,
            CurrentMotion::Stationary
        ));
        assert_eq!(controller.last_anchor_depth(), -9_000.0);
        assert!((transform.translation.length() - 9_000.0).abs() < 0.01);
        assert!(!controller.enabled_motion.pan);
    }

    fn camera_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f64(1.0 / 60.0),
            ))
            .init_resource::<BodyFrame>()
            .init_resource::<CameraFocusState>()
            .init_resource::<SelectedSatellite>()
            .init_resource::<NoradIndex>()
            .init_resource::<LaunchLibraryUiState>()
            .add_message::<CameraRequest>()
            .add_systems(Update, finalize_camera);
        let camera = app
            .world_mut()
            .spawn((
                MainCamera,
                orbit_controller(25_000.0),
                Transform::from_xyz(0.0, 0.0, 25_000.0),
            ))
            .id();
        (app, camera)
    }

    #[test]
    fn tracking_uses_ecef_and_cancels_when_target_is_removed() {
        let (mut app, camera) = camera_app();
        let satellite = app
            .world_mut()
            .spawn((
                Satellite,
                WorldEcefKm(DVec3::new(7_000.0, 0.0, 0.0)),
                Transform::from_xyz(1.0, 2.0, 3.0),
            ))
            .id();
        app.world_mut()
            .resource_mut::<NoradIndex>()
            .map
            .insert(42, satellite);
        app.world_mut().resource_mut::<SelectedSatellite>().selected = Some(42);
        app.world_mut().resource_mut::<SelectedSatellite>().tracking = Some(42);
        app.world_mut().resource_mut::<CameraFocusState>().target = CameraFocusTarget::Moon;
        app.world_mut()
            .resource_mut::<LaunchLibraryUiState>()
            .camera_target = Some(CameraPose {
            radius: 9_000.0,
            yaw: 1.0,
            pitch: 0.0,
        });
        app.update();
        assert_eq!(
            app.world().resource::<CameraFocusState>().target,
            CameraFocusTarget::Earth
        );
        assert!(
            app.world()
                .resource::<LaunchLibraryUiState>()
                .camera_target
                .is_none()
        );
        let before = app.world().get::<Transform>(camera).unwrap().translation;
        assert!(before.distance(Vec3::new(0.0, 0.0, 12_000.0)) < 0.1);
        app.world_mut().despawn(satellite);
        app.update();
        assert!(
            app.world()
                .resource::<SelectedSatellite>()
                .tracking
                .is_none()
        );
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            before
        );
    }

    #[test]
    fn focus_toggle_cancels_tracking_and_restores_earth_pose() {
        let (mut app, camera) = camera_app();
        app.world_mut()
            .insert_resource(MoonEcefKm(DVec3::new(380_000.0, 0.0, 0.0)));
        app.world_mut().resource_mut::<SelectedSatellite>().tracking = Some(42);
        app.world_mut()
            .resource_mut::<Messages<CameraRequest>>()
            .write(CameraRequest::ToggleFocus);
        app.update();
        assert_eq!(
            app.world().resource::<CameraFocusState>().target,
            CameraFocusTarget::Moon
        );
        assert!(
            app.world()
                .resource::<SelectedSatellite>()
                .tracking
                .is_none()
        );
        assert!(
            (app.world()
                .get::<Transform>(camera)
                .unwrap()
                .translation
                .distance(Vec3::new(0.0, 0.0, 380_000.0))
                - 10_000.0)
                .abs()
                < 0.1
        );
        app.world_mut()
            .resource_mut::<Messages<CameraRequest>>()
            .write(CameraRequest::ToggleFocus);
        app.update();
        assert_eq!(
            app.world().resource::<CameraFocusState>().target,
            CameraFocusTarget::Earth
        );
        assert!(
            app.world()
                .get::<Transform>(camera)
                .unwrap()
                .translation
                .distance(Vec3::new(0.0, 0.0, 25_000.0))
                < 0.1
        );
    }

    #[test]
    fn moon_camera_system_transports_between_simulation_frames() {
        let (mut app, camera) = camera_app();
        app.world_mut()
            .insert_resource(MoonEcefKm(DVec3::new(380_000.0, 0.0, 0.0)));
        let moon = app.world_mut().spawn((Moon, Transform::default())).id();
        app.world_mut()
            .resource_mut::<Messages<CameraRequest>>()
            .write(CameraRequest::ToggleFocus);
        app.update();
        let old_center = Vec3::new(0.0, 0.0, 380_000.0);
        let old_offset = app.world().get::<Transform>(camera).unwrap().translation - old_center;
        let new_center = Vec3::new(20_000.0, 10_000.0, 370_000.0);
        app.world_mut().resource_mut::<MoonEcefKm>().0 =
            crate::core::space::bevy_to_ecef_km(new_center);
        let rotation = Quat::from_rotation_y(0.4);
        app.world_mut().get_mut::<Transform>(moon).unwrap().rotation = rotation;
        app.update();
        let local = rotation.inverse()
            * (app.world().get::<Transform>(camera).unwrap().translation - new_center);
        assert!(local.distance(old_offset) < 0.1);
    }

    #[test]
    fn launch_fly_to_completes_without_restarting_from_popup_updates() {
        let (mut app, camera) = camera_app();
        let launch = crate::launch_library::LaunchSummary {
            id: Some(1),
            name: "Fixture launch".into(),
            net_utc: None,
            pad_id: None,
            pad_name: None,
            pad_lat: Some(28.5),
            pad_lon: Some(-80.6),
            pad_location_name: None,
            provider_name: None,
            mission_name: None,
            orbit_name: None,
        };
        app.world_mut().insert_resource(LaunchLibraryData {
            launches: vec![launch],
            events: vec![],
        });
        app.world_mut().resource_mut::<SelectedSatellite>().tracking = Some(42);
        app.world_mut()
            .resource_mut::<LaunchLibraryUiState>()
            .selection = Some(LaunchLibrarySelection {
            kind: LaunchLibraryItemKind::Launch,
            index: 0,
        });
        app.update();
        assert!(
            app.world()
                .resource::<SelectedSatellite>()
                .tracking
                .is_none()
        );
        assert!(
            app.world()
                .resource::<LaunchLibraryUiState>()
                .camera_target
                .is_some()
        );
        for _ in 0..180 {
            app.update();
        }
        assert!(
            app.world()
                .resource::<LaunchLibraryUiState>()
                .camera_target
                .is_none()
        );
        assert!(
            (app.world()
                .get::<Transform>(camera)
                .unwrap()
                .translation
                .length()
                - 9_000.0)
                .abs()
                < 5.0
        );
        // A stable popup selection must not recreate an already completed move.
        app.update();
        assert!(
            app.world()
                .resource::<LaunchLibraryUiState>()
                .camera_target
                .is_none()
        );
    }
}
