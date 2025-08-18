//! Coordinate transformation utilities for orbital mechanics

use bevy::math::{DVec3, Vec3};
use chrono::{DateTime, Utc, Datelike, Timelike};

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
pub fn gmst_rad(t: DateTime<Utc>) -> f64 {
    let jd = julian_date_utc(t);
    let t_cent = (jd - 2451545.0) / 36525.0; // Julian centuries from J2000.0

    // GMST in seconds (IAU 1982 with update terms). See Vallado and IERS Conventions.
    let gmst_sec = 67310.54841
        + (876600.0 * 3600.0 + 8640184.812866) * t_cent
        + 0.093104 * t_cent * t_cent
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

    let gmst_sec = 67310.54841
        + (876600.0 * 3600.0 + 8640184.812866) * t_cent
        + 0.093104 * t_cent * t_cent
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_gmst_rad_j2000_known_value() {
        // Reference: GMST at J2000.0 (2000-01-01 12:00:00 UT1) is 18.697374558 hours
        // = 280.46061837 degrees
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

        // With no rotation, should be the same
        assert!((ecef.x - 1000.0).abs() < 1e-10);
        assert!(ecef.y.abs() < 1e-10);
        assert!(ecef.z.abs() < 1e-10);

        // Test with 90 degree rotation (corrected for -GMST rotation)
        let gmst_90 = std::f64::consts::PI / 2.0;
        let ecef_90 = eci_to_ecef_km(eci, gmst_90);

        // With corrected rotation: X should become Y, Y should become -X
        assert!(ecef_90.x.abs() < 1e-10);
        assert!((ecef_90.y + 1000.0).abs() < 1e-10);
        assert!(ecef_90.z.abs() < 1e-10);
    }

    #[test]
    fn test_geostationary_satellite_ecef_stability() {
        // Test that a geostationary satellite position remains stable in ECEF
        // Using approximate GEO altitude (35,786 km above Earth surface)
        let geo_radius_km = 6371.0 + 35786.0; // Earth radius + GEO altitude
        
        // Initial ECI position at 0° longitude (X-axis in ECI at GMST=0)
        let eci_initial = DVec3::new(geo_radius_km, 0.0, 0.0);
        let gmst_0 = 0.0;
        let ecef_0 = eci_to_ecef_km(eci_initial, gmst_0);
        
        // After 6 hours (GMST advances by π/2 radians)
        // For a geostationary satellite, ECI position should rotate with Earth
        let gmst_6h = std::f64::consts::PI / 2.0;
        let eci_6h = DVec3::new(0.0, geo_radius_km, 0.0); // Rotated 90° in ECI
        let ecef_6h = eci_to_ecef_km(eci_6h, gmst_6h);
        
        // ECEF positions should be nearly identical for geostationary orbit
        let position_diff = (ecef_0 - ecef_6h).length();
        assert!(position_diff < 1.0, "GEO satellite ECEF position changed by {} km", position_diff);
        
        // After 12 hours (GMST advances by π radians)
        let gmst_12h = std::f64::consts::PI;
        let eci_12h = DVec3::new(-geo_radius_km, 0.0, 0.0); // Rotated 180° in ECI
        let ecef_12h = eci_to_ecef_km(eci_12h, gmst_12h);
        
        let position_diff_12h = (ecef_0 - ecef_12h).length();
        assert!(position_diff_12h < 1.0, "GEO satellite ECEF position changed by {} km after 12h", position_diff_12h);
    }

    #[test]
    fn test_julian_date_j2000_noon() {
        // JD at 2000-01-01 12:00:00 UTC should be exactly 2451545.0
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
        // ECI +X at GMST=0 maps to ECEF +X
        // Bevy remap should place it on +Z axis
        let eci = DVec3::new(1000.0, 0.0, 0.0);
        let ecef = eci_to_ecef_km(eci, 0.0);
        let bevy = ecef_to_bevy_world_km(ecef);

        assert!(bevy.x.abs() < 1e-6);
        assert!(bevy.y.abs() < 1e-6);
        assert!((bevy.z - 1000.0).abs() < 1e-6);
    }
}
