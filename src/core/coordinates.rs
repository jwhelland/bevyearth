//! Core coordinate utilities
//!
//! Unifies all coordinate-related types and functions into a single module:
//! - Geographic coordinates and helpers (was in crate::coord)
//! - Orbital/Earth-frame transformations and time utilities (was in crate::orbital::coordinates)
//!
//! Recommended usage:
//! - Import from crate::core::coordinates::*
//! - Legacy paths via crate::coord and crate::orbital::coordinates are temporarily re-exported as shims.

use bevy::math::{DVec3, Vec3};
use chrono::{DateTime, Datelike, Timelike, Utc};
use std::f64::consts::PI;

pub const EARTH_RADIUS_KM: f32 = 6371.0;

// ========================= Geographic coordinates and helpers =========================

#[derive(Debug)]
#[allow(dead_code)]
pub struct CoordError {
    pub msg: String,
}

#[derive(Debug)]
pub struct Coordinates {
    // Stored internally in radians (f64 for precision)
    pub latitude: f64,
    pub longitude: f64,
}

impl From<Vec3> for Coordinates {
    fn from(value: Vec3) -> Self {
        let n = value.normalize();
        let y = n.y as f64;
        let x = n.x as f64;
        let z = n.z as f64;
        let latitude = y.asin();
        let longitude = x.atan2(z);
        Coordinates {
            latitude,
            longitude,
        }
    }
}

impl Coordinates {
    pub fn as_degrees(&self) -> (f32, f32) {
        let latitude = (self.latitude * (180.0_f64 / PI)) as f32;
        let longitude = (self.longitude * (180.0_f64 / PI)) as f32;
        (latitude, longitude)
    }

    pub fn convert_to_uv_mercator(&self) -> (f32, f32) {
        let (lat, lon) = self.as_degrees();
        let v = map_latitude(lat).unwrap();
        let u = map_longitude(lon).unwrap();
        (u, v)
    }

    #[allow(dead_code)]
    pub fn from_degrees(latitude: f32, longitude: f32) -> Result<Self, CoordError> {
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(CoordError {
                msg: format!("Invalid latitude: {:?}", latitude),
            });
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(CoordError {
                msg: format!("Invalid longitude: {:?}", longitude),
            });
        }
        let latitude = (latitude as f64) / (180.0_f64 / PI);
        let longitude = (longitude as f64) / (180.0_f64 / PI);
        Ok(Coordinates {
            latitude,
            longitude,
        })
    }

    pub fn get_point_on_sphere(&self) -> Vec3 {
        // Compute with f64 for pole precision, then cast to f32 (Bevy uses f32)
        let lat = self.latitude;
        let lon = self.longitude;
        let y = lat.sin();
        let mut r = lat.cos();
        // Clamp residual radius near the poles to avoid mm-scale artifacts from f32 quantization of 90°
        if (std::f64::consts::FRAC_PI_2 - lat.abs()).abs() < 1e-7 {
            r = 0.0;
        }
        let x = lon.sin() * r;
        let z = lon.cos() * r;
        Vec3::new(x as f32, y as f32, z as f32) * EARTH_RADIUS_KM
    }
}

// High-precision map helper
fn map64((in_min, in_max): (f64, f64), (out_min, out_max): (f64, f64), value: f64) -> f64 {
    let denom = in_max - in_min;
    if denom.abs() < f64::EPSILON {
        out_min
    } else {
        (value - in_min) / denom * (out_max - out_min) + out_min
    }
}

// Maps a value from one range to another (f32 API, f64 math)
#[allow(dead_code)]
fn map((in_min, in_max): (f32, f32), (out_min, out_max): (f32, f32), value: f32) -> f32 {
    map64(
        (in_min as f64, in_max as f64),
        (out_min as f64, out_max as f64),
        value as f64,
    ) as f32
}

fn map_latitude(lat: f32) -> Result<f32, CoordError> {
    // 90 -> 0 maps to 0.0 to 0.5
    // 0 -> -90 maps to 0.5 to 1.0
    // Ensure latitude is valid
    if !(-90.0..=90.0).contains(&lat) {
        return Err(CoordError {
            msg: format!("Invalid latitude: {:?}", lat),
        });
    }
    let lat64 = lat as f64;
    let v = if (0.0..=90.0).contains(&lat) {
        map64((90.0, 0.0), (0.0, 0.5), lat64)
    } else {
        map64((0.0, -90.0), (0.5, 1.0), lat64)
    };
    Ok(v as f32)
}

fn map_longitude(lon: f32) -> Result<f32, CoordError> {
    // -180 -> 0 maps to 0.0 to 0.5
    // 0 -> 180 maps to 0.5 to 1.0
    // Ensure longitude is valid
    if !(-180.0..=180.0).contains(&lon) {
        return Err(CoordError {
            msg: format!("Invalid longitude: {:?}", lon),
        });
    }
    let lon64 = lon as f64;
    let u = if (-180.0..=0.0).contains(&lon) {
        map64((-180.0, 0.0), (0.0, 0.5), lon64)
    } else {
        map64((0.0, 180.0), (0.5, 1.0), lon64)
    };
    Ok(u as f32)
}

/// True if the straight segment from city (on/near sphere surface) to satellite does NOT intersect the Earth sphere.
/// Uses a robust segment-sphere intersection test around the origin.
pub fn los_visible_ecef(city_ecef_km: Vec3, sat_ecef_km: Vec3, earth_radius_km: f32) -> bool {
    // Promote to f64 for numerical robustness
    let c = DVec3::new(
        city_ecef_km.x as f64,
        city_ecef_km.y as f64,
        city_ecef_km.z as f64,
    );
    let s = DVec3::new(
        sat_ecef_km.x as f64,
        sat_ecef_km.y as f64,
        sat_ecef_km.z as f64,
    );
    let u = s - c;

    // Solve |C + t u|^2 = R^2  -> (u·u) t^2 + 2 (C·u) t + (C·C - R^2) = 0
    let a = u.length_squared();
    if a == 0.0 {
        // City and satellite at same point -> degenerate, treat as not visible
        return false;
    }
    let b = 2.0_f64 * c.dot(u);
    let r2 = (earth_radius_km as f64) * (earth_radius_km as f64);
    let c_term = c.length_squared() - r2;

    let discr = b * b - 4.0_f64 * a * c_term;

    if discr < 0.0 {
        // No intersection with infinite line => segment cannot hit sphere
        return true;
    }

    let sqrt_d = discr.sqrt();
    let t1 = (-b - sqrt_d) / (2.0_f64 * a);
    let t2 = (-b + sqrt_d) / (2.0_f64 * a);

    // Exclude grazing at the city endpoint: require t > eps (in km units).
    let eps: f64 = 1e-5_f64; // 1e-5 km = 1 cm
    // If either intersection parameter lies within (eps, 1], LOS is blocked.
    let hits_segment = ((t1 > eps) && (t1 <= 1.0)) || ((t2 > eps) && (t2 <= 1.0));
    !hits_segment
}

/// Cheap prefilter: city is potentially visible only if city and satellite are on the same hemisphere
/// relative to the sphere origin. Equivalent to dot(C, S) > R^2 (both outside the tangent plane).
pub fn hemisphere_prefilter(city_ecef_km: Vec3, sat_ecef_km: Vec3, earth_radius_km: f32) -> bool {
    let c = DVec3::new(
        city_ecef_km.x as f64,
        city_ecef_km.y as f64,
        city_ecef_km.z as f64,
    );
    let s = DVec3::new(
        sat_ecef_km.x as f64,
        sat_ecef_km.y as f64,
        sat_ecef_km.z as f64,
    );
    c.dot(s) > (earth_radius_km as f64) * (earth_radius_km as f64)
}

// ========================= Orbital/Earth-frame transformations =========================

/// Compute the Julian Date (UTC) for a given timestamp.
/// Uses the standard Gregorian calendar to JD conversion.
pub fn julian_date_utc(t: DateTime<Utc>) -> f64 {
    let mut y = t.year();
    let mut m = t.month() as i32;
    let d = t.day() as i32;

    // Convert time of day to fraction of day
    let hour = t.hour() as f64;
    let minute = t.minute() as f64;
    let sec = t.second() as f64 + (t.nanosecond() as f64) * 1e-9_f64;
    let day_fraction = (hour + (minute + sec / 60.0) / 60.0) / 24.0;

    if m <= 2 {
        y -= 1;
        m += 12;
    }

    let a = (y as f64 / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();

    let jd0 = (365.25 * (y as f64 + 4716.0)).floor()
        + (30.6001 * ((m + 1) as f64)).floor()
        + d as f64
        + b
        - 1524.5;

    jd0 + day_fraction
}

/// Greenwich Mean Sidereal Time (radians) using IAU 1982/2006 polynomial.
/// Assumes UT1 ~= UTC (good enough for visualization; allows optional DUT1 later).
#[allow(dead_code)]
pub fn gmst_rad(t: DateTime<Utc>) -> f64 {
    let jd = julian_date_utc(t);
    let t_cent = (jd - 2451545.0) / 36525.0; // Julian centuries from J2000.0

    // GMST in seconds (IAU 1982 with update terms). See Vallado and IERS Conventions.
    let gmst_sec =
        67310.54841 + (876600.0 * 3600.0 + 8640184.812866) * t_cent + 0.093104 * t_cent * t_cent
            - 6.2e-6 * t_cent * t_cent * t_cent;

    // Normalize to [0, 86400)
    let sec_in_day = 86400.0_f64;
    let mut s = gmst_sec % sec_in_day;
    if s < 0.0 {
        s += sec_in_day;
    }

    s * (std::f64::consts::TAU / sec_in_day)
}

/// Rotate ECI (TEME) -> ECEF using simple GMST rotation about Z
/// Standard transformation rotates by -GMST (clockwise when viewed from +Z)
pub fn eci_to_ecef_km(eci: DVec3, gmst: f64) -> DVec3 {
    let (s, c) = gmst.sin_cos();
    let x = c * eci.x + s * eci.y;
    let y = -s * eci.x + c * eci.y;
    DVec3::new(x, y, eci.z)
}

/// Greenwich Mean Sidereal Time (radians) allowing explicit DUT1 (UT1-UTC) seconds.
/// If `dut1_seconds` is 0, this is equivalent to `gmst_rad`.
pub fn gmst_rad_with_dut1(t: DateTime<Utc>, dut1_seconds: f64) -> f64 {
    let jd_utc = julian_date_utc(t);
    let jd_ut1 = jd_utc + dut1_seconds / 86400.0_f64;
    let t_cent = (jd_ut1 - 2451545.0) / 36525.0; // Julian centuries from J2000.0

    let gmst_sec =
        67310.54841 + (876600.0 * 3600.0 + 8640184.812866) * t_cent + 0.093104 * t_cent * t_cent
            - 6.2e-6 * t_cent * t_cent * t_cent;

    let sec_in_day = 86400.0_f64;
    let mut s = gmst_sec % sec_in_day;
    if s < 0.0 {
        s += sec_in_day;
    }
    s * (std::f64::consts::TAU / sec_in_day)
}

/// Remap ECEF axes to Bevy world coordinates in kilometers.
/// Mapping: Bevy (x,y,z) = (ECEF.y, ECEF.z, ECEF.x)
pub fn ecef_to_bevy_world_km(ecef: DVec3) -> Vec3 {
    Vec3::new(ecef.y as f32, ecef.z as f32, ecef.x as f32)
}

// =================================== Tests ===================================

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Vec3 as BVec3;
    use chrono::TimeZone;

    const EPSILON: f32 = 1e-6;

    // ---- Geographic coordinate tests (from former crate::coord) ----

    #[test]
    fn test_coordinates_from_degrees_valid() {
        let coord = Coordinates::from_degrees(45.0, 90.0).unwrap();
        let (lat_deg, lon_deg) = coord.as_degrees();

        assert!((lat_deg - 45.0).abs() < EPSILON);
        assert!((lon_deg - 90.0).abs() < EPSILON);
    }

    #[test]
    fn test_coordinates_from_degrees_boundary_values() {
        assert!(Coordinates::from_degrees(90.0, 180.0).is_ok());
        assert!(Coordinates::from_degrees(-90.0, -180.0).is_ok());
        assert!(Coordinates::from_degrees(0.0, 0.0).is_ok());
    }

    #[test]
    fn test_coordinates_from_degrees_invalid_latitude() {
        assert!(Coordinates::from_degrees(91.0, 0.0).is_err());
        assert!(Coordinates::from_degrees(-91.0, 0.0).is_err());
    }

    #[test]
    fn test_coordinates_from_degrees_invalid_longitude() {
        assert!(Coordinates::from_degrees(0.0, 181.0).is_err());
        assert!(Coordinates::from_degrees(0.0, -181.0).is_err());
    }

    #[test]
    fn test_coordinates_as_degrees() {
        let coord = Coordinates {
            latitude: PI / 4.0,
            longitude: PI / 2.0,
        };
        let (lat_deg, lon_deg) = coord.as_degrees();

        assert!((lat_deg - 45.0).abs() < EPSILON);
        assert!((lon_deg - 90.0).abs() < EPSILON);
    }

    #[test]
    fn test_coordinates_from_vec3() {
        let vec = BVec3::new(0.0, 1.0, 0.0); // North pole
        let coord = Coordinates::from(vec);
        let (lat_deg, lon_deg) = coord.as_degrees();

        assert!((lat_deg - 90.0).abs() < EPSILON);
        assert!(lon_deg.is_finite());
    }

    #[test]
    fn test_coordinates_from_vec3_equator() {
        let vec = BVec3::new(1.0, 0.0, 0.0);
        let coord = Coordinates::from(vec);
        let (lat_deg, lon_deg) = coord.as_degrees();

        assert!((lat_deg - 0.0).abs() < EPSILON);
        assert!((lon_deg - 90.0).abs() < EPSILON);
    }

    #[test]
    fn test_get_point_on_sphere() {
        let coord = Coordinates::from_degrees(0.0, 0.0).unwrap(); // Equator, prime meridian
        let point = coord.get_point_on_sphere();

        // Should be on the sphere surface
        assert!((point.length() - EARTH_RADIUS_KM).abs() < EPSILON);

        // Should be at (0, 0, EARTH_RADIUS_KM) in Bevy coordinates
        assert!((point.x - 0.0).abs() < EPSILON);
        assert!((point.y - 0.0).abs() < EPSILON);
        assert!((point.z - EARTH_RADIUS_KM).abs() < EPSILON);
    }

    #[test]
    fn test_get_point_on_sphere_north_pole() {
        let coord = Coordinates::from_degrees(90.0, 0.0).unwrap();
        let point = coord.get_point_on_sphere();

        assert!((point.length() - EARTH_RADIUS_KM).abs() < EPSILON);
        assert!((point.y - EARTH_RADIUS_KM).abs() < EPSILON);
        assert!(point.x.abs() < EPSILON);
        assert!(point.z.abs() < EPSILON);
    }

    #[test]
    fn test_map_function() {
        let result = map((0.0, 10.0), (0.0, 100.0), 5.0);
        assert!((result - 50.0).abs() < EPSILON);

        let result = map((-1.0, 1.0), (0.0, 1.0), 0.0);
        assert!((result - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_map_latitude_valid() {
        let result = map_latitude(90.0).unwrap();
        assert!((result - 0.0).abs() < EPSILON);

        let result = map_latitude(0.0).unwrap();
        assert!((result - 0.5).abs() < EPSILON);

        let result = map_latitude(-90.0).unwrap();
        assert!((result - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_map_latitude_invalid() {
        assert!(map_latitude(91.0).is_err());
        assert!(map_latitude(-91.0).is_err());
    }

    #[test]
    fn test_map_longitude_valid() {
        let result = map_longitude(-180.0).unwrap();
        assert!((result - 0.0).abs() < EPSILON);

        let result = map_longitude(0.0).unwrap();
        assert!((result - 0.5).abs() < EPSILON);

        let result = map_longitude(180.0).unwrap();
        assert!((result - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_map_longitude_invalid() {
        assert!(map_longitude(181.0).is_err());
        assert!(map_longitude(-181.0).is_err());
    }

    #[test]
    fn test_convert_to_uv_mercator() {
        let coord = Coordinates::from_degrees(0.0, 0.0).unwrap();
        let (u, v) = coord.convert_to_uv_mercator();
        assert!((u - 0.5).abs() < EPSILON);
        assert!((v - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_los_visible_ecef_clear_line_of_sight() {
        let city = BVec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        let satellite = BVec3::new(0.0, 0.0, EARTH_RADIUS_KM * 2.0);
        assert!(los_visible_ecef(city, satellite, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_los_visible_ecef_blocked_by_earth() {
        let city = BVec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        let satellite = BVec3::new(0.0, 0.0, -EARTH_RADIUS_KM * 2.0);
        assert!(!los_visible_ecef(city, satellite, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_los_visible_ecef_same_position() {
        let position = BVec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        assert!(!los_visible_ecef(position, position, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_los_visible_ecef_high_satellite() {
        let city = BVec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        let satellite = BVec3::new(0.0, 0.0, EARTH_RADIUS_KM * 10.0);
        assert!(los_visible_ecef(city, satellite, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_los_visible_ecef_grazing_case() {
        let city = BVec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        let satellite = BVec3::new(EARTH_RADIUS_KM * 2.0, 0.0, EARTH_RADIUS_KM);
        assert!(los_visible_ecef(city, satellite, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_hemisphere_prefilter_same_hemisphere() {
        let city = BVec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        let satellite = BVec3::new(100.0, 100.0, EARTH_RADIUS_KM * 2.0);
        assert!(hemisphere_prefilter(city, satellite, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_hemisphere_prefilter_opposite_hemispheres() {
        let city = BVec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        let satellite = BVec3::new(0.0, 0.0, -EARTH_RADIUS_KM * 2.0);
        assert!(!hemisphere_prefilter(city, satellite, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_hemisphere_prefilter_edge_case() {
        let city = BVec3::new(EARTH_RADIUS_KM, 0.0, 0.0);
        let satellite = BVec3::new(0.0, EARTH_RADIUS_KM, 0.0);
        let result = hemisphere_prefilter(city, satellite, EARTH_RADIUS_KM);
        assert!(!result);
    }

    #[test]
    fn test_roundtrip_conversion() {
        let original = BVec3::new(1.0, 1.0, 1.0).normalize();
        let coord = Coordinates::from(original);
        let reconstructed = coord.get_point_on_sphere().normalize();
        let diff = (original - reconstructed).length();
        assert!(diff < 1e-5);
    }

    #[test]
    fn test_coordinates_debug_format() {
        let coord = Coordinates::from_degrees(45.0, 90.0).unwrap();
        let debug_str = format!("{:?}", coord);
        assert!(debug_str.contains("Coordinates"));
    }

    #[test]
    fn test_coord_error_debug_format() {
        let error = CoordError {
            msg: "Test error".to_string(),
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("CoordError"));
        assert!(debug_str.contains("Test error"));
    }

    #[test]
    fn test_coordinates_extreme_longitude_values() {
        let coord_near_180 = Coordinates::from_degrees(0.0, 179.999999).unwrap();
        let (_, lon) = coord_near_180.as_degrees();
        assert!((lon - 179.999999).abs() < EPSILON);

        let coord_near_minus_180 = Coordinates::from_degrees(0.0, -179.999999).unwrap();
        let (_, lon2) = coord_near_minus_180.as_degrees();
        assert!((lon2 - (-179.999999)).abs() < EPSILON);
    }

    #[test]
    fn test_coordinates_extreme_latitude_values() {
        let coord_near_north_pole = Coordinates::from_degrees(89.999999, 0.0).unwrap();
        let (lat, _) = coord_near_north_pole.as_degrees();
        assert!((lat - 89.999999).abs() < EPSILON);

        let coord_near_south_pole = Coordinates::from_degrees(-89.999999, 0.0).unwrap();
        let (lat2, _) = coord_near_south_pole.as_degrees();
        assert!((lat2 - (-89.999999)).abs() < EPSILON);
    }

    #[test]
    fn test_coordinates_international_date_line() {
        let coord_180 = Coordinates::from_degrees(0.0, 180.0).unwrap();
        let coord_minus_180 = Coordinates::from_degrees(0.0, -180.0).unwrap();

        let (_, lon_180) = coord_180.as_degrees();
        let (_, lon_minus_180) = coord_minus_180.as_degrees();

        assert!((lon_180 - 180.0).abs() < EPSILON);
        assert!((lon_minus_180 - (-180.0)).abs() < EPSILON);

        let point_180 = coord_180.get_point_on_sphere();
        let point_minus_180 = coord_minus_180.get_point_on_sphere();
        let diff = (point_180 - point_minus_180).length();
        assert!(
            diff < 0.01,
            "Points at ±180° should be very close, diff: {}",
            diff
        );
    }

    #[test]
    fn test_coordinates_prime_meridian_and_equator_intersection() {
        let coord = Coordinates::from_degrees(0.0, 0.0).unwrap();
        let point = coord.get_point_on_sphere();
        assert!((point.x - 0.0).abs() < EPSILON);
        assert!((point.y - 0.0).abs() < EPSILON);
        assert!((point.z - EARTH_RADIUS_KM).abs() < EPSILON);

        let (u, v) = coord.convert_to_uv_mercator();
        assert!((u - 0.5).abs() < EPSILON);
        assert!((v - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_coordinates_antipodal_points() {
        let coord1 = Coordinates::from_degrees(45.0, 90.0).unwrap();
        let coord2 = Coordinates::from_degrees(-45.0, -90.0).unwrap();

        let point1 = coord1.get_point_on_sphere();
        let point2 = coord2.get_point_on_sphere();

        let distance = (point1 - point2).length();
        let expected_distance = 2.0 * EARTH_RADIUS_KM;
        assert!(
            (distance - expected_distance).abs() < 1e-3,
            "Antipodal distance should be {}, got {}",
            expected_distance,
            distance
        );
    }

    #[test]
    fn test_coordinates_precision_near_poles() {
        let north_pole = Coordinates::from_degrees(90.0, 0.0).unwrap();
        let north_pole_diff_lon = Coordinates::from_degrees(90.0, 180.0).unwrap();

        let point1 = north_pole.get_point_on_sphere();
        let point2 = north_pole_diff_lon.get_point_on_sphere();

        let diff = (point1 - point2).length();
        assert!(
            diff < 1e-2,
            "North pole positions with different longitudes should be very close, diff: {}",
            diff
        );
    }

    #[test]
    fn test_map_function_edge_cases() {
        let result = map((0.0, 1.0), (0.0, 1.0), 0.5);
        assert!((result - 0.5).abs() < EPSILON);

        let result = map((0.0, 1.0), (1.0, 0.0), 0.25);
        assert!((result - 0.75).abs() < EPSILON);

        let result = map((5.0, 5.0), (0.0, 10.0), 5.0);
        assert!(result.is_finite());
        assert!((result - 0.0).abs() < EPSILON);
    }

    #[test]
    fn test_map_latitude_edge_values() {
        let result = map_latitude(90.0).unwrap();
        assert!((result - 0.0).abs() < EPSILON);

        let result = map_latitude(-90.0).unwrap();
        assert!((result - 1.0).abs() < EPSILON);

        let result = map_latitude(89.9999).unwrap();
        assert!(result < 0.01);

        let result = map_latitude(-89.9999).unwrap();
        assert!(result > 0.99);
    }

    #[test]
    fn test_map_longitude_edge_values() {
        let result = map_longitude(-180.0).unwrap();
        assert!((result - 0.0).abs() < EPSILON);

        let result = map_longitude(180.0).unwrap();
        assert!((result - 1.0).abs() < EPSILON);

        let result = map_longitude(-179.9999).unwrap();
        assert!(result < 0.01);

        let result = map_longitude(179.9999).unwrap();
        assert!(result > 0.99);
    }

    #[test]
    fn test_los_visible_ecef_edge_cases() {
        let city = BVec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        let satellite_above = BVec3::new(0.0, 0.0, EARTH_RADIUS_KM * 2.0);
        assert!(los_visible_ecef(city, satellite_above, EARTH_RADIUS_KM));

        let satellite_very_far = BVec3::new(0.0, 0.0, EARTH_RADIUS_KM * 100.0);
        assert!(los_visible_ecef(city, satellite_very_far, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_hemisphere_prefilter_edge_cases() {
        let city = BVec3::new(EARTH_RADIUS_KM, 0.0, 0.0);
        let satellite = BVec3::new(0.0, EARTH_RADIUS_KM, 0.0);
        let result = hemisphere_prefilter(city, satellite, EARTH_RADIUS_KM);
        assert!(!result);

        let satellite_above = BVec3::new(EARTH_RADIUS_KM * 1.1, EARTH_RADIUS_KM * 1.1, 0.0);
        let result_above = hemisphere_prefilter(city, satellite_above, EARTH_RADIUS_KM);
        assert!(result_above);

        let zero = BVec3::ZERO;
        let result_zero = hemisphere_prefilter(zero, zero, EARTH_RADIUS_KM);
        assert!(!result_zero);
    }

    #[test]
    fn test_roundtrip_conversion_precision() {
        let test_coords = vec![
            (0.0, 0.0),
            (90.0, 0.0),
            (-90.0, 0.0),
            (45.0, 90.0),
            (-45.0, -90.0),
            (0.0, 180.0),
            (89.9, 179.9),
            (-89.9, -179.9),
        ];

        for (lat, lon) in test_coords {
            let original_coord = Coordinates::from_degrees(lat, lon).unwrap();
            let point = original_coord.get_point_on_sphere();
            let reconstructed_coord = Coordinates::from(point.normalize());

            let (orig_lat, orig_lon) = original_coord.as_degrees();
            let (recon_lat, recon_lon) = reconstructed_coord.as_degrees();

            // Allow larger tolerance near poles where longitude is less meaningful
            let lat_tolerance = if lat.abs() > 89.0 { 1e-2 } else { 1e-4 };
            let lon_tolerance = if lat.abs() > 89.0 { 1.0 } else { 1e-4 };

            assert!(
                (orig_lat - recon_lat).abs() < lat_tolerance,
                "Latitude roundtrip failed for ({}, {}): {} vs {}",
                lat,
                lon,
                orig_lat,
                recon_lat
            );

            if lat.abs() < 89.0 {
                // Only check longitude away from poles
                // Handle longitude wraparound at ±180°
                let mut lon_diff = (orig_lon - recon_lon).abs();
                if lon_diff > 180.0 {
                    lon_diff = 360.0 - lon_diff;
                }
                if (lat, lon) == (0.0, 180.0) || (lat, lon) == (0.0, -180.0) {
                    assert!(lon_diff < 1.0);
                } else {
                    assert!(lon_diff < lon_tolerance);
                }
            }
        }
    }

    // ---- Orbital/ECEF/Bevy transform tests (from former crate::orbital::coordinates) ----

    #[test]
    fn test_gmst_rad_j2000_known_value() {
        // Reference: GMST at J2000.0 (2000-01-01 12:00:00 UT1) is 18.697374558 hours = 280.46061837 deg
        let t = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
        let gmst = gmst_rad(t);
        let deg = gmst.to_degrees();
        let expected_deg = 280.46061837_f64;
        let diff = (deg - expected_deg).abs();
        assert!(diff < 0.05, "gmst deg diff too large: {} deg", diff);
    }

    #[test]
    fn test_eci_to_ecef_km() {
        let eci = DVec3::new(1000.0, 0.0, 0.0);
        let gmst = 0.0; // No rotation
        let ecef = eci_to_ecef_km(eci, gmst);

        assert!((ecef.x - 1000.0).abs() < 1e-10);
        assert!(ecef.y.abs() < 1e-10);
        assert!(ecef.z.abs() < 1e-10);

        let gmst_90 = std::f64::consts::PI / 2.0;
        let ecef_90 = eci_to_ecef_km(eci, gmst_90);

        assert!(ecef_90.x.abs() < 1e-10);
        assert!((ecef_90.y + 1000.0).abs() < 1e-10);
        assert!(ecef_90.z.abs() < 1e-10);
    }

    #[test]
    fn test_geostationary_satellite_ecef_stability() {
        let geo_radius_km = 6371.0 + 35786.0;
        let eci_initial = DVec3::new(geo_radius_km, 0.0, 0.0);
        let gmst_0 = 0.0;
        let ecef_0 = eci_to_ecef_km(eci_initial, gmst_0);

        let gmst_6h = std::f64::consts::PI / 2.0;
        let eci_6h = DVec3::new(0.0, geo_radius_km, 0.0);
        let ecef_6h = eci_to_ecef_km(eci_6h, gmst_6h);

        let position_diff = (ecef_0 - ecef_6h).length();
        assert!(position_diff < 1.0, "GEO changed by {} km", position_diff);

        let gmst_12h = std::f64::consts::PI;
        let eci_12h = DVec3::new(-geo_radius_km, 0.0, 0.0);
        let ecef_12h = eci_to_ecef_km(eci_12h, gmst_12h);

        let position_diff_12h = (ecef_0 - ecef_12h).length();
        assert!(
            position_diff_12h < 1.0,
            "GEO changed by {} km after 12h",
            position_diff_12h
        );
    }

    #[test]
    fn test_julian_date_j2000_noon() {
        let t = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
        let jd = julian_date_utc(t);
        assert!((jd - 2451545.0).abs() < 1e-9, "jd = {}", jd);
    }

    #[test]
    fn test_gmst_with_dut1_matches_zero_offset() {
        let t = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let a = gmst_rad(t);
        let b = gmst_rad_with_dut1(t, 0.0);
        assert!((a - b).abs() < 1e-12);
    }

    #[test]
    fn test_ecef_to_bevy_axis_mapping_at_gmst_zero() {
        let eci = DVec3::new(1000.0, 0.0, 0.0);
        let ecef = eci_to_ecef_km(eci, 0.0);
        let bevy = ecef_to_bevy_world_km(ecef);

        assert!(bevy.x.abs() < 1e-6);
        assert!(bevy.y.abs() < 1e-6);
        assert!((bevy.z - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn test_julian_date_leap_year_boundaries() {
        let leap_feb_28 = Utc.with_ymd_and_hms(2000, 2, 28, 12, 0, 0).unwrap();
        let leap_feb_29 = Utc.with_ymd_and_hms(2000, 2, 29, 12, 0, 0).unwrap();
        let leap_mar_01 = Utc.with_ymd_and_hms(2000, 3, 1, 12, 0, 0).unwrap();

        let jd_feb_28 = julian_date_utc(leap_feb_28);
        let jd_feb_29 = julian_date_utc(leap_feb_29);
        let jd_mar_01 = julian_date_utc(leap_mar_01);

        assert!((jd_feb_29 - jd_feb_28 - 1.0).abs() < 1e-9);
        assert!((jd_mar_01 - jd_feb_29 - 1.0).abs() < 1e-9);

        let leap_2004_feb_29 = Utc.with_ymd_and_hms(2004, 2, 29, 12, 0, 0).unwrap();
        let leap_2004_mar_01 = Utc.with_ymd_and_hms(2004, 3, 1, 12, 0, 0).unwrap();

        let jd_2004_feb_29 = julian_date_utc(leap_2004_feb_29);
        let jd_2004_mar_01 = julian_date_utc(leap_2004_mar_01);

        assert!((jd_2004_mar_01 - jd_2004_feb_29 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_julian_date_century_boundaries() {
        let century_1900 = Utc.with_ymd_and_hms(1900, 1, 1, 12, 0, 0).unwrap();
        let jd_1900 = julian_date_utc(century_1900);

        let century_2000 = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
        let jd_2000 = julian_date_utc(century_2000);

        let expected_days = 36524.0;
        let actual_diff = jd_2000 - jd_1900;
        assert!(
            (actual_diff - expected_days).abs() < 1e-6,
            "Century difference should be {} days, got {}",
            expected_days,
            actual_diff
        );

        let century_1600 = Utc.with_ymd_and_hms(1600, 1, 1, 12, 0, 0).unwrap();
        let jd_1600 = julian_date_utc(century_1600);
        assert!(jd_1600 > 2000000.0 && jd_1600 < 2500000.0);
    }

    #[test]
    fn test_gmst_leap_year_consistency() {
        let before_leap = Utc.with_ymd_and_hms(2000, 2, 28, 23, 59, 59).unwrap();
        let after_leap = Utc.with_ymd_and_hms(2000, 3, 1, 0, 0, 1).unwrap();

        let gmst_before = gmst_rad(before_leap);
        let gmst_after = gmst_rad(after_leap);

        let mut gmst_diff = gmst_after - gmst_before;
        while gmst_diff < 0.0 {
            gmst_diff += std::f64::consts::TAU;
        }
        while gmst_diff >= std::f64::consts::TAU {
            gmst_diff -= std::f64::consts::TAU;
        }

        assert!(gmst_diff > 0.0 && gmst_diff < std::f64::consts::TAU);
    }

    #[test]
    fn test_gmst_century_boundary_precision() {
        let century_2000 = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
        let century_2100 = Utc.with_ymd_and_hms(2100, 1, 1, 12, 0, 0).unwrap();

        let gmst_2000 = gmst_rad(century_2000);
        let gmst_2100 = gmst_rad(century_2100);

        assert!(gmst_2000 >= 0.0 && gmst_2000 < std::f64::consts::TAU);
        assert!(gmst_2100 >= 0.0 && gmst_2100 < std::f64::consts::TAU);
        assert!(gmst_2000.is_finite() && gmst_2100.is_finite());

        let century_1900 = Utc.with_ymd_and_hms(1900, 1, 1, 12, 0, 0).unwrap();
        let gmst_1900 = gmst_rad(century_1900);
        assert!(gmst_1900 >= 0.0 && gmst_1900 < std::f64::consts::TAU);
        assert!(gmst_1900.is_finite());
    }

    #[test]
    fn test_gmst_with_dut1_edge_cases() {
        let test_time = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();

        let gmst_plus_dut1 = gmst_rad_with_dut1(test_time, 0.9);
        let gmst_minus_dut1 = gmst_rad_with_dut1(test_time, -0.9);
        let gmst_zero_dut1 = gmst_rad_with_dut1(test_time, 0.0);

        assert!(gmst_plus_dut1 >= 0.0 && gmst_plus_dut1 < std::f64::consts::TAU);
        assert!(gmst_minus_dut1 >= 0.0 && gmst_minus_dut1 < std::f64::consts::TAU);

        let diff_plus = (gmst_plus_dut1 - gmst_zero_dut1).abs();
        let diff_minus = (gmst_minus_dut1 - gmst_zero_dut1).abs();

        let expected_diff = 0.9 * std::f64::consts::TAU / 86400.0;
        assert!(diff_plus < expected_diff * 2.0);
        assert!(diff_minus < expected_diff * 2.0);
        assert!(diff_plus > expected_diff * 0.5);
        assert!(diff_minus > expected_diff * 0.5);
    }

    #[test]
    fn test_eci_to_ecef_full_rotation_cycle() {
        let eci_point = DVec3::new(7000.0, 0.0, 0.0);

        let gmst_values = [
            0.0,
            std::f64::consts::PI / 4.0,
            std::f64::consts::PI / 2.0,
            3.0 * std::f64::consts::PI / 4.0,
            std::f64::consts::PI,
            5.0 * std::f64::consts::PI / 4.0,
            3.0 * std::f64::consts::PI / 2.0,
            7.0 * std::f64::consts::PI / 4.0,
            std::f64::consts::TAU,
        ];

        let mut ecef_positions = Vec::new();
        for &gmst in &gmst_values {
            let ecef = eci_to_ecef_km(eci_point, gmst);
            ecef_positions.push(ecef);

            assert!(
                (ecef.length() - eci_point.length()).abs() < 1e-10,
                "ECEF distance should preserve ECI distance"
            );
        }

        let diff = (ecef_positions[0] - ecef_positions[8]).length();
        assert!(
            diff < 1e-10,
            "0° and 360° rotations should be identical, diff: {}",
            diff
        );

        let ecef_0 = ecef_positions[0];
        let ecef_180 = ecef_positions[4];
        assert!(
            (ecef_0.x + ecef_180.x).abs() < 1e-10,
            "180° rotation should flip X"
        );
        assert!(
            (ecef_0.y + ecef_180.y).abs() < 1e-10,
            "180° rotation should flip Y"
        );
        assert!(
            (ecef_0.z - ecef_180.z).abs() < 1e-10,
            "180° rotation should preserve Z"
        );
    }

    #[test]
    fn test_eci_to_ecef_precision_with_small_angles() {
        let eci = DVec3::new(6371.0, 1000.0, 500.0);

        let small_angles = [1e-10, 1e-8, 1e-6, 1e-4, 1e-2];

        for &angle in &small_angles {
            let ecef = eci_to_ecef_km(eci, angle);

            // For small angles, cos(θ) ≈ 1, sin(θ) ≈ θ
            let expected_x = eci.x + angle * eci.y;
            let expected_y = -angle * eci.x + eci.y;

            let x_error = (ecef.x - expected_x).abs();
            let y_error = (ecef.y - expected_y).abs();

            // Error should be proportional to angle²
            let expected_error = angle * angle * eci.length() * 0.5;
            assert!(x_error < expected_error + 1e-12);
            assert!(y_error < expected_error + 1e-12);
        }
    }

    #[test]
    fn test_ecef_to_bevy_coordinate_system_consistency() {
        let ecef_x = DVec3::new(1000.0, 0.0, 0.0);
        let ecef_y = DVec3::new(0.0, 1000.0, 0.0);
        let ecef_z = DVec3::new(0.0, 0.0, 1000.0);

        let bevy_x = ecef_to_bevy_world_km(ecef_x);
        let bevy_y = ecef_to_bevy_world_km(ecef_y);
        let bevy_z = ecef_to_bevy_world_km(ecef_z);

        assert!((bevy_x.x - 0.0).abs() < 1e-6);
        assert!((bevy_x.y - 0.0).abs() < 1e-6);
        assert!((bevy_x.z - 1000.0).abs() < 1e-6);

        assert!((bevy_y.x - 1000.0).abs() < 1e-6);
        assert!((bevy_y.y - 0.0).abs() < 1e-6);
        assert!((bevy_y.z - 0.0).abs() < 1e-6);

        assert!((bevy_z.x - 0.0).abs() < 1e-6);
        assert!((bevy_z.y - 1000.0).abs() < 1e-6);
        assert!((bevy_z.z - 0.0).abs() < 1e-6);

        let ecef_diagonal = DVec3::new(100.0, 200.0, 300.0);
        let bevy_diagonal = ecef_to_bevy_world_km(ecef_diagonal);

        let ecef_length = ecef_diagonal.length();
        let bevy_length = bevy_diagonal.length() as f64;
        assert!((ecef_length - bevy_length).abs() < 1e-3);
    }
}
