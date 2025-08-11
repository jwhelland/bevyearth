//! Footprint gizmo rendering for satellite coverage visualization
//!
//! This module provides simple, highly visible circle rendering using Bevy gizmos
//! instead of complex meshes for better visibility on Earth's surface.

use bevy::prelude::*;
use std::f32::consts::PI;

use crate::coverage::{CoverageParameters, FootprintCalculator, FootprintConfig};
use crate::earth::EARTH_RADIUS_KM;
use crate::satellite::{Satellite, SatelliteStore};

/// Plugin for footprint gizmo rendering and management
pub struct FootprintGizmoPlugin;

impl Plugin for FootprintGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FootprintGizmoConfig>()
            .add_systems(
                Update,
                (
                    manage_footprint_gizmo_components_system,
                    draw_footprint_gizmos_system.after(manage_footprint_gizmo_components_system),
                ),
            );
    }
}

/// System to manage footprint gizmo components (add/remove based on settings)
fn manage_footprint_gizmo_components_system(
    mut commands: Commands,
    mut store: ResMut<SatelliteStore>,
    footprint_config: Res<FootprintConfig>,
    gizmo_config: Res<FootprintGizmoConfig>,
    satellite_query: Query<Entity, With<Satellite>>,
    gizmo_query: Query<Entity, With<FootprintGizmo>>,
) {
    if !footprint_config.enabled || !gizmo_config.enabled {
        // If footprints are globally disabled, remove all FootprintGizmo components
        for gizmo_entity in gizmo_query.iter() {
            commands.entity(gizmo_entity).remove::<FootprintGizmo>();
        }
        return;
    }

    for entry in store.items.values_mut() {
        let should_show = entry.show_footprint && entry.propagator.is_some();
        
        if let Some(sat_entity) = entry.entity {
            if let Ok(entity) = satellite_query.get(sat_entity) {
                let has_gizmo_component = gizmo_query.get(entity).is_ok();
                
                if should_show && !has_gizmo_component {
                    // Add FootprintGizmo component (parameters will be read dynamically from UI)
                    let dummy_params = CoverageParameters::default();
                    commands.entity(entity).insert(FootprintGizmo::new(entry.norad, dummy_params));
                } else if !should_show && has_gizmo_component {
                    // Remove FootprintGizmo component
                    commands.entity(entity).remove::<FootprintGizmo>();
                }
            }
        }
    }
}

/// Component marker for satellites that should show footprint gizmos
#[derive(Component)]
pub struct FootprintGizmo {
    /// NORAD ID of the associated satellite
    #[allow(dead_code)]
    pub satellite_norad: u32,
    /// Coverage parameters for this footprint
    #[allow(dead_code)]
    pub coverage_params: CoverageParameters,
    /// Whether to show the footprint
    pub enabled: bool,
}

impl FootprintGizmo {
    pub fn new(satellite_norad: u32, coverage_params: CoverageParameters) -> Self {
        Self {
            satellite_norad,
            coverage_params,
            enabled: true,
        }
    }
}

/// Configuration for footprint gizmo rendering
#[derive(Resource)]
pub struct FootprintGizmoConfig {
    /// Global enable/disable for all footprint gizmos
    pub enabled: bool,
    /// Number of segments for circle approximation
    pub circle_segments: u32,
    /// Line width for the footprint circle
    #[allow(dead_code)]
    pub line_width: f32,
    /// Color for the footprint circle
    pub circle_color: Color,
    /// Whether to draw multiple concentric circles for signal strength zones
    pub show_signal_zones: bool,
    /// Colors for different signal strength zones (strong to weak)
    pub zone_colors: Vec<Color>,
    /// Whether to draw a center dot at the nadir point
    pub show_center_dot: bool,
    /// Size of the center dot
    pub center_dot_size: f32,
}

impl Default for FootprintGizmoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            circle_segments: 64,
            line_width: 2.0,
            circle_color: Color::srgba(0.0, 1.0, 0.0, 1.0), // Bright green, fully opaque
            show_signal_zones: false, // Start with simple single circle
            zone_colors: vec![
                Color::srgba(0.0, 1.0, 0.0, 1.0), // Strong signal - bright green
                Color::srgba(0.5, 1.0, 0.0, 0.9), // Medium signal - yellow-green
                Color::srgba(1.0, 1.0, 0.0, 0.8), // Weak signal - yellow
                Color::srgba(1.0, 0.5, 0.0, 0.7), // Very weak signal - orange
            ],
            show_center_dot: true,
            center_dot_size: 200.0, // km
        }
    }
}

/// System to draw footprint gizmos for satellites
pub fn draw_footprint_gizmos_system(
    mut gizmos: Gizmos,
    config: Res<FootprintGizmoConfig>,
    footprint_config: Res<crate::coverage::FootprintConfig>,
    satellite_query: Query<(&Transform, &FootprintGizmo), With<Satellite>>,
) {
    if !config.enabled || !footprint_config.enabled {
        return;
    }

    for (transform, footprint_gizmo) in satellite_query.iter() {
        if !footprint_gizmo.enabled {
            continue;
        }

        // Use current UI parameters instead of cached ones
        let current_params = crate::coverage::CoverageParameters {
            frequency_mhz: footprint_config.default_frequency_mhz,
            transmit_power_dbm: footprint_config.default_tx_power_dbm,
            antenna_gain_dbi: footprint_config.default_antenna_gain_dbi,
            min_signal_strength_dbm: footprint_config.default_min_signal_dbm,
            min_elevation_deg: footprint_config.default_min_elevation_deg,
        };

        // Debug: Print the parameters being used for coverage calculation
        println!("[GIZMO] Using parameters: freq={:.1} MHz, power={:.1} dBm, gain={:.1} dBi, min_signal={:.1} dBm, min_elev={:.1}°",
                 current_params.frequency_mhz,
                 current_params.transmit_power_dbm,
                 current_params.antenna_gain_dbi,
                 current_params.min_signal_strength_dbm,
                 current_params.min_elevation_deg);

        let sat_pos = transform.translation;
        draw_satellite_footprint_gizmo(
            &mut gizmos,
            &config,
            sat_pos,
            &current_params,
        );
    }
}

/// Draw a single satellite's footprint as gizmo circles
fn draw_satellite_footprint_gizmo(
    gizmos: &mut Gizmos,
    config: &FootprintGizmoConfig,
    sat_ecef_km: Vec3,
    coverage_params: &CoverageParameters,
) {
    // Calculate satellite altitude
    let sat_altitude_km = sat_ecef_km.length() - EARTH_RADIUS_KM;
    
    // Calculate coverage radius on Earth's surface
    let surface_radius_km = FootprintCalculator::calculate_surface_coverage_radius(
        sat_altitude_km,
        coverage_params,
        EARTH_RADIUS_KM,
    );

    // If no coverage, don't draw anything
    if surface_radius_km <= 0.0 {
        return;
    }

    // Find the nadir point (ground projection of satellite)
    let nadir_point = sat_ecef_km.normalize() * EARTH_RADIUS_KM;

    // Create local coordinate system at nadir point
    let up = nadir_point.normalize();
    let right = if up.y.abs() < 0.9 {
        up.cross(Vec3::Y).normalize()
    } else {
        up.cross(Vec3::X).normalize()
    };
    let forward = right.cross(up);

    // Draw center dot if enabled
    if config.show_center_dot {
        draw_center_dot(gizmos, nadir_point, up, right, forward, config.center_dot_size, config.circle_color);
    }

    // Draw signal strength zones or single circle
    if config.show_signal_zones && config.zone_colors.len() > 1 {
        draw_signal_zones(
            gizmos,
            nadir_point,
            up,
            right,
            forward,
            surface_radius_km,
            &config.zone_colors,
            config.circle_segments,
        );
    } else {
        draw_footprint_circle(
            gizmos,
            nadir_point,
            up,
            right,
            forward,
            surface_radius_km,
            config.circle_color,
            config.circle_segments,
        );
    }
}

/// Draw a circle on the Earth's surface
fn draw_footprint_circle(
    gizmos: &mut Gizmos,
    center: Vec3,
    _up: Vec3,
    right: Vec3,
    forward: Vec3,
    radius_km: f32,
    color: Color,
    segments: u32,
) {
    let angle_step = 2.0 * PI / segments as f32;
    let mut points = Vec::with_capacity(segments as usize);

    for i in 0..segments {
        let angle = i as f32 * angle_step;
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
        gizmos.line(points[i as usize], points[next_i as usize], color);
    }
}

/// Draw multiple concentric circles for signal strength zones
fn draw_signal_zones(
    gizmos: &mut Gizmos,
    center: Vec3,
    up: Vec3,
    right: Vec3,
    forward: Vec3,
    max_radius_km: f32,
    zone_colors: &[Color],
    segments: u32,
) {
    let num_zones = zone_colors.len();
    
    for (zone_idx, &color) in zone_colors.iter().enumerate() {
        // Calculate radius for this zone (from outer to inner)
        let zone_fraction = (zone_idx + 1) as f32 / num_zones as f32;
        let zone_radius = max_radius_km * zone_fraction;
        
        draw_footprint_circle(
            gizmos,
            center,
            up,
            right,
            forward,
            zone_radius,
            color,
            segments,
        );
    }
}

/// Draw a small circle at the nadir point
fn draw_center_dot(
    gizmos: &mut Gizmos,
    center: Vec3,
    up: Vec3,
    right: Vec3,
    forward: Vec3,
    dot_size_km: f32,
    color: Color,
) {
    draw_footprint_circle(
        gizmos,
        center,
        up,
        right,
        forward,
        dot_size_km,
        color,
        16, // Lower resolution for center dot
    );
}

/// Project a point onto the Earth's sphere surface
fn project_to_sphere_surface(point: Vec3) -> Vec3 {
    point.normalize() * EARTH_RADIUS_KM
}

/// Utility functions for footprint gizmo management
#[allow(dead_code)]
pub struct FootprintGizmoUtils;

impl FootprintGizmoUtils {
    /// Check if a satellite position would produce a visible footprint
    #[allow(dead_code)]
    pub fn has_visible_coverage(
        sat_ecef_km: Vec3,
        coverage_params: &CoverageParameters,
    ) -> bool {
        let sat_altitude_km = sat_ecef_km.length() - EARTH_RADIUS_KM;
        let surface_radius_km = FootprintCalculator::calculate_surface_coverage_radius(
            sat_altitude_km,
            coverage_params,
            EARTH_RADIUS_KM,
        );
        surface_radius_km > 0.0
    }

    /// Calculate the footprint radius for a satellite
    #[allow(dead_code)]
    pub fn calculate_footprint_radius(
        sat_ecef_km: Vec3,
        coverage_params: &CoverageParameters,
    ) -> f32 {
        let sat_altitude_km = sat_ecef_km.length() - EARTH_RADIUS_KM;
        FootprintCalculator::calculate_surface_coverage_radius(
            sat_altitude_km,
            coverage_params,
            EARTH_RADIUS_KM,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_footprint_gizmo_creation() {
        let params = CoverageParameters::default();
        let gizmo = FootprintGizmo::new(12345, params.clone());
        
        assert_eq!(gizmo.satellite_norad, 12345);
        assert!(gizmo.enabled);
        assert_eq!(gizmo.coverage_params.frequency_mhz, params.frequency_mhz);
    }

    #[test]
    fn test_coverage_visibility() {
        let sat_pos = Vec3::new(0.0, 0.0, EARTH_RADIUS_KM + 550.0);
        let params = CoverageParameters::default();

        assert!(FootprintGizmoUtils::has_visible_coverage(sat_pos, &params));
    }

    #[test]
    fn test_footprint_radius_calculation() {
        let sat_pos = Vec3::new(0.0, 0.0, EARTH_RADIUS_KM + 550.0);
        let params = CoverageParameters::default();
        
        let radius = FootprintGizmoUtils::calculate_footprint_radius(sat_pos, &params);
        assert!(radius > 0.0);
        assert!(radius < EARTH_RADIUS_KM); // Should be reasonable
    }

    #[test]
    fn test_frequency_affects_footprint_radius() {
        let sat_pos = Vec3::new(0.0, 0.0, EARTH_RADIUS_KM + 550.0);

        let mut params_low = CoverageParameters::default();
        params_low.frequency_mhz = 1000.0; // Lower frequency
        params_low.min_elevation_deg = 0.0; // Disable elevation limit

        let mut params_high = CoverageParameters::default();
        params_high.frequency_mhz = 2000.0; // Higher frequency
        params_high.min_elevation_deg = 0.0; // Disable elevation limit

        let radius_low = FootprintGizmoUtils::calculate_footprint_radius(sat_pos, &params_low);
        let radius_high = FootprintGizmoUtils::calculate_footprint_radius(sat_pos, &params_high);

        println!("Radius at low frequency (1000 MHz): {:.3} km", radius_low);
        println!("Radius at high frequency (2000 MHz): {:.3} km", radius_high);

        // Assert that the radius changes with frequency, without assuming which is larger
        assert_ne!(radius_low, radius_high, "Footprint radius should change with frequency");
    }
}