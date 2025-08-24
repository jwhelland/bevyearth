//! Arrow visualization systems

use crate::core::coordinates::{EARTH_RADIUS_KM, hemisphere_prefilter, los_visible_ecef};
use crate::satellite::{Satellite, SatelliteColor};
use crate::visualization::CitiesEcef;
use crate::visualization::config::ArrowConfig;
use bevy::prelude::*;

/// Draw arrow segment from city to satellite
pub fn draw_arrow_segment(
    gizmos: &mut Gizmos,
    city: Vec3,
    sat_pos: Vec3,
    fallback_color: Color,
    config: &ArrowConfig,
) {
    // constants conversion meters->kilometers
    let lift_km = config.lift_m / 1000.0;
    let head_min_km = config.head_min_m / 1000.0;
    let head_max_km = config.head_max_m / 1000.0;
    // Direction and lifted city endpoint
    let dir = (sat_pos - city).normalize();
    let city_lifted = city.normalize() * (EARTH_RADIUS_KM + lift_km);
    let total_len = (sat_pos - city_lifted).length();

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

    let shaft_end = city_lifted + dir * shaft_len;
    gizmos.arrow(city_lifted, shaft_end, draw_color);

    let _ = (head_min_km, head_max_km); // reserved for potential arrowhead
}

/// System to draw arrows from cities to satellites
pub fn draw_city_to_satellite_arrows(
    mut gizmos: Gizmos,
    sat_query: Query<(&Transform, Option<&SatelliteColor>), With<Satellite>>,
    cities: Option<Res<CitiesEcef>>,
    config: Res<ArrowConfig>,
) {
    if !config.enabled {
        return;
    }
    let Some(cities) = cities else { return };
    let mut sats: Vec<(Vec3, Color)> = Vec::new();
    for (t, color_comp) in sat_query.iter() {
        let color = color_comp.map(|c| c.0).unwrap_or(config.color);
        sats.push((t.translation, color));
    }
    if sats.is_empty() {
        return;
    }

    let mut drawn = 0usize;
    'outer: for &city in cities.iter() {
        for &(sat_pos, sat_color) in &sats {
            if !hemisphere_prefilter(city, sat_pos, EARTH_RADIUS_KM) {
                continue;
            }
            if !los_visible_ecef(city, sat_pos, EARTH_RADIUS_KM) {
                continue;
            }
            draw_arrow_segment(&mut gizmos, city, sat_pos, sat_color, &config);
            drawn += 1;
            if drawn >= config.max_visible {
                break 'outer;
            }
        }
    }
}
