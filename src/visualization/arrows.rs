//! Arrow visualization systems

use crate::core::big_space::{BigSpaceRoot, ecef_to_render, render_origin_from_grid};
use crate::core::coordinates::{
    EARTH_RADIUS_KM, hemisphere_prefilter_ecef_dvec, los_visible_ecef_dvec,
};
use crate::core::space::WorldEcefKm;
use crate::satellite::{Satellite, SatelliteColor};
use crate::visualization::CitiesEcef;
use crate::visualization::config::ArrowConfig;
use bevy::math::DVec3;
use bevy::prelude::*;
use big_space::prelude::{CellCoord, Grid};

/// Draw arrow segment from city to satellite
pub fn draw_arrow_segment(
    gizmos: &mut Gizmos,
    city_ecef_km: DVec3,
    sat_ecef_km: DVec3,
    fallback_color: Color,
    config: &ArrowConfig,
    grid: &Grid,
    origin_cell: CellCoord,
    origin_local: Vec3,
) {
    // constants conversion meters->kilometers
    let lift_km = config.lift_m as f64 / 1000.0;
    // Direction and lifted city endpoint
    let dir = (sat_ecef_km - city_ecef_km).normalize();
    let city_lifted = city_ecef_km.normalize() * (EARTH_RADIUS_KM as f64 + lift_km);
    let total_len = (sat_ecef_km - city_lifted).length() as f32;

    // color gradient
    let draw_color = if config.gradient_enabled {
        let mut near = config.gradient_near_km.max(1e-3);
        let mut far = config.gradient_far_km.max(near + 1e-3);
        if near > far {
            core::mem::swap(&mut near, &mut far);
        }
        let t = if config.gradient_log_scale {
            let ln = |x: f32| x.max(1e-3).ln();
            ((ln(total_len) - ln(near)) / (ln(far) - ln(near))).clamp(0.0, 1.0)
        } else {
            ((total_len - near) / (far - near)).clamp(0.0, 1.0)
        };
        config
            .gradient_near_color
            .mix(&config.gradient_far_color, t)
    } else {
        fallback_color
    };

    let mut shaft_len = config.shaft_len_pct * total_len;
    let shaft_min_km = config.shaft_min_m / 1000.0;
    let shaft_max_km = config.shaft_max_m / 1000.0;
    shaft_len = shaft_len
        .clamp(shaft_min_km, shaft_max_km)
        .min(total_len * 0.9);

    let shaft_end = city_lifted + dir * shaft_len as f64;
    let city_lifted_bevy = ecef_to_render(grid, city_lifted, origin_cell, origin_local);
    let shaft_end_bevy = ecef_to_render(grid, shaft_end, origin_cell, origin_local);
    gizmos.arrow(city_lifted_bevy, shaft_end_bevy, draw_color);
}

/// System to draw arrows from cities to satellites
pub fn draw_city_to_satellite_arrows(
    mut gizmos: Gizmos,
    sat_query: Query<(&WorldEcefKm, Option<&SatelliteColor>), With<Satellite>>,
    cities: Option<Res<CitiesEcef>>,
    config: Res<ArrowConfig>,
    big_space_root: Res<BigSpaceRoot>,
    grid_query: Query<&Grid>,
) {
    if !config.enabled {
        return;
    }
    let Ok(grid) = grid_query.get(big_space_root.0) else {
        return;
    };
    let (origin_cell, origin_local) = render_origin_from_grid(grid);
    let Some(cities) = cities else { return };
    let mut sats: Vec<(DVec3, Color)> = Vec::new();
    for (world_ecef, color_comp) in sat_query.iter() {
        let color = color_comp.map(|c| c.0).unwrap_or(config.color);
        sats.push((world_ecef.0, color));
    }
    if sats.is_empty() {
        return;
    }

    let mut drawn = 0usize;
    let earth_radius_km = EARTH_RADIUS_KM as f64;
    'outer: for &city_ecef in cities.iter() {
        for &(sat_ecef, sat_color) in &sats {
            if !hemisphere_prefilter_ecef_dvec(city_ecef, sat_ecef, earth_radius_km) {
                continue;
            }
            if !los_visible_ecef_dvec(city_ecef, sat_ecef, earth_radius_km) {
                continue;
            }
            draw_arrow_segment(
                &mut gizmos,
                city_ecef,
                sat_ecef,
                sat_color,
                &config,
                grid,
                origin_cell,
                origin_local,
            );
            drawn += 1;
            if drawn >= config.max_visible {
                break 'outer;
            }
        }
    }
}
