//! Ground track gizmo rendering for satellite visualization
//!
//! This module provides simple, highly visible circle rendering using Bevy gizmos
//! for better visibility on Earth's surface.

use bevy::prelude::*;
use std::f64::consts::PI;

use crate::core::coordinates::EARTH_RADIUS_KM;
use crate::core::space::{WorldEcefKm, ecef_to_bevy_km};
use crate::satellite::{Satellite, SatelliteStore};
use bevy::math::DVec3;

/// Plugin for ground track gizmo rendering and management
pub struct GroundTrackGizmoPlugin;

impl Plugin for GroundTrackGizmoPlugin {
    fn build(&self, app: &mut App) {
        // GroundTrackGizmoConfig is now initialized in UiConfigBundle
        app.add_systems(
            Update,
            (
                manage_ground_track_gizmo_components_system,
                draw_ground_track_gizmos_system.after(manage_ground_track_gizmo_components_system),
            ),
        );
    }
}

/// System to manage ground track gizmo components (add/remove based on settings)
fn manage_ground_track_gizmo_components_system(
    mut commands: Commands,
    mut store: ResMut<SatelliteStore>,
    config_bundle: Res<crate::ui::systems::UiConfigBundle>,
    satellite_query: Query<Entity, With<Satellite>>,
    gizmo_query: Query<Entity, With<GroundTrackGizmo>>,
) {
    if !config_bundle.ground_track_cfg.enabled || !config_bundle.gizmo_cfg.enabled {
        // If ground tracks are globally disabled, remove all GroundTrackGizmo components
        for gizmo_entity in gizmo_query.iter() {
            commands.entity(gizmo_entity).remove::<GroundTrackGizmo>();
        }
        return;
    }

    for entry in store.items.values_mut() {
        let should_show = entry.show_ground_track && entry.propagator.is_some();

        if let Some(sat_entity) = entry.entity
            && let Ok(entity) = satellite_query.get(sat_entity)
        {
            let has_gizmo_component = gizmo_query.get(entity).is_ok();

            if should_show && !has_gizmo_component {
                // Add GroundTrackGizmo component
                commands
                    .entity(entity)
                    .insert(GroundTrackGizmo::new(entry.norad));
            } else if !should_show && has_gizmo_component {
                // Remove GroundTrackGizmo component
                commands.entity(entity).remove::<GroundTrackGizmo>();
            }
        }
    }
}

/// Component marker for satellites that should show ground track gizmos
#[derive(Component)]
pub struct GroundTrackGizmo {
    /// NORAD ID of the associated satellite
    #[allow(dead_code)]
    pub satellite_norad: u32,
    /// Whether to show the ground track
    pub enabled: bool,
}

impl GroundTrackGizmo {
    pub fn new(satellite_norad: u32) -> Self {
        Self {
            satellite_norad,
            enabled: true,
        }
    }
}

/// Configuration for ground track gizmo rendering
#[derive(Resource)]
pub struct GroundTrackGizmoConfig {
    /// Global enable/disable for all ground track gizmos
    pub enabled: bool,
    /// Number of segments for circle approximation
    pub circle_segments: u32,
    /// Color for the ground track circle
    pub circle_color: Color,
    /// Whether to draw a center dot at the nadir point
    pub show_center_dot: bool,
    /// Size of the center dot
    pub center_dot_size: f32,
}

impl Default for GroundTrackGizmoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            circle_segments: 64,
            circle_color: Color::srgba(0.0, 1.0, 1.0, 0.8), // Cyan
            show_center_dot: true,
            center_dot_size: 25.0, // km
        }
    }
}

/// System to draw ground track gizmos for satellites
pub fn draw_ground_track_gizmos_system(
    mut gizmos: Gizmos,
    config_bundle: Res<crate::ui::systems::UiConfigBundle>,
    satellite_query: Query<(&WorldEcefKm, &GroundTrackGizmo), With<Satellite>>,
) {
    if !config_bundle.gizmo_cfg.enabled || !config_bundle.ground_track_cfg.enabled {
        return;
    }

    for (world_ecef, ground_track_gizmo) in satellite_query.iter() {
        if !ground_track_gizmo.enabled {
            continue;
        }

        draw_satellite_ground_track_gizmo(
            &mut gizmos,
            &config_bundle.gizmo_cfg,
            world_ecef.0,
            config_bundle.ground_track_cfg.radius_km,
        );
    }
}

/// Draw a single satellite's ground track as gizmo circles
fn draw_satellite_ground_track_gizmo(
    gizmos: &mut Gizmos,
    config: &GroundTrackGizmoConfig,
    sat_ecef_km: DVec3,
    radius_km: f32,
) {
    // Find the nadir point (ground projection of satellite)
    let nadir_point = sat_ecef_km.normalize() * (EARTH_RADIUS_KM as f64);

    // Create local coordinate system at nadir point
    let up = nadir_point.normalize();
    let right = if up.y.abs() < 0.9 {
        up.cross(DVec3::Y).normalize()
    } else {
        up.cross(DVec3::X).normalize()
    };
    let forward = right.cross(up);

    // Draw center dot if enabled
    if config.show_center_dot {
        draw_center_dot(
            gizmos,
            nadir_point,
            right,
            forward,
            config.center_dot_size as f64,
            config.circle_color,
        );
    }

    draw_ground_track_circle(
        gizmos,
        nadir_point,
        right,
        forward,
        radius_km as f64,
        config.circle_color,
        config.circle_segments,
    );
}

/// Draw a circle on the Earth's surface
fn draw_ground_track_circle(
    gizmos: &mut Gizmos,
    center: DVec3,
    right: DVec3,
    forward: DVec3,
    radius_km: f64,
    color: Color,
    segments: u32,
) {
    let angle_step = 2.0 * PI / segments as f64;
    let mut points = Vec::with_capacity(segments as usize);

    for i in 0..segments {
        let angle = i as f64 * angle_step;
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();

        // Calculate position on Earth's surface
        let local_offset = right * cos_angle + forward * sin_angle;
        let surface_point = project_to_sphere_surface(center + local_offset * radius_km);
        points.push(surface_point);
    }

    // Draw the circle as connected line segments
    for i in 0..segments {
        let next_i = (i + 1) % segments;
        let p0 = ecef_to_bevy_km(points[i as usize]);
        let p1 = ecef_to_bevy_km(points[next_i as usize]);
        gizmos.line(p0, p1, color);
    }
}

/// Draw a small circle at the nadir point
fn draw_center_dot(
    gizmos: &mut Gizmos,
    center: DVec3,
    right: DVec3,
    forward: DVec3,
    dot_size_km: f64,
    color: Color,
) {
    draw_ground_track_circle(
        gizmos,
        center,
        right,
        forward,
        dot_size_km,
        color,
        16, // Lower resolution for center dot
    );
}

/// Project a point onto the Earth's sphere surface
fn project_to_sphere_surface(point: DVec3) -> DVec3 {
    point.normalize() * (EARTH_RADIUS_KM as f64)
}
