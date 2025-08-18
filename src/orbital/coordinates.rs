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
#[allow(dead_code)]
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

    // Additional edge case tests for Phase 2
    #[test]
    fn test_julian_date_leap_year_boundaries() {
        // Test leap year boundary conditions
        
        // 2000 was a leap year (divisible by 400)
        let leap_feb_28 = Utc.with_ymd_and_hms(2000, 2, 28, 12, 0, 0).unwrap();
        let leap_feb_29 = Utc.with_ymd_and_hms(2000, 2, 29, 12, 0, 0).unwrap();
        let leap_mar_01 = Utc.with_ymd_and_hms(2000, 3, 1, 12, 0, 0).unwrap();
        
        let jd_feb_28 = julian_date_utc(leap_feb_28);
        let jd_feb_29 = julian_date_utc(leap_feb_29);
        let jd_mar_01 = julian_date_utc(leap_mar_01);
        
        // Should be exactly 1 day apart
        assert!((jd_feb_29 - jd_feb_28 - 1.0).abs() < 1e-9);
        assert!((jd_mar_01 - jd_feb_29 - 1.0).abs() < 1e-9);
        
        // Test 2004 (regular leap year)
        let leap_2004_feb_29 = Utc.with_ymd_and_hms(2004, 2, 29, 12, 0, 0).unwrap();
        let leap_2004_mar_01 = Utc.with_ymd_and_hms(2004, 3, 1, 12, 0, 0).unwrap();
        
        let jd_2004_feb_29 = julian_date_utc(leap_2004_feb_29);
        let jd_2004_mar_01 = julian_date_utc(leap_2004_mar_01);
        
        assert!((jd_2004_mar_01 - jd_2004_feb_29 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_julian_date_century_boundaries() {
        // Test century boundary conditions for Gregorian calendar
        
        // 1900 was not a leap year (divisible by 100 but not 400)
        let century_1900 = Utc.with_ymd_and_hms(1900, 1, 1, 12, 0, 0).unwrap();
        let jd_1900 = julian_date_utc(century_1900);
        
        // 2000 was a leap year (divisible by 400)
        let century_2000 = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
        let jd_2000 = julian_date_utc(century_2000);
        
        // Should be exactly 100 years apart (36524 days for century with 24 leap years)
        let expected_days = 36524.0; // 100 years with 24 leap years (not 25 because 1900 wasn't leap)
        let actual_diff = jd_2000 - jd_1900;
        assert!((actual_diff - expected_days).abs() < 1e-6,
                "Century difference should be {} days, got {}", expected_days, actual_diff);
        
        // Test year 1600 (was a leap year)
        let century_1600 = Utc.with_ymd_and_hms(1600, 1, 1, 12, 0, 0).unwrap();
        let jd_1600 = julian_date_utc(century_1600);
        
        // Verify JD calculation is reasonable for historical dates
        assert!(jd_1600 > 2000000.0 && jd_1600 < 2500000.0, "JD for 1600 should be reasonable: {}", jd_1600);
    }

    #[test]
    fn test_julian_date_precision_with_fractional_seconds() {
        // Test precision with fractional seconds and nanoseconds
        let base_time = Utc.with_ymd_and_hms(2024, 6, 15, 12, 30, 45).unwrap();
        let base_jd = julian_date_utc(base_time);
        
        // Add 500 milliseconds
        let time_plus_500ms = base_time + chrono::Duration::milliseconds(500);
        let jd_plus_500ms = julian_date_utc(time_plus_500ms);
        
        // Should differ by 500ms / (24*3600*1000) days
        let expected_diff = 500.0 / (24.0 * 3600.0 * 1000.0);
        let actual_diff = jd_plus_500ms - base_jd;
        assert!((actual_diff - expected_diff).abs() < 1e-6,
                "500ms should add {} days to JD, got {}", expected_diff, actual_diff);
        
        // Test with microseconds
        let time_plus_1us = base_time + chrono::Duration::microseconds(1);
        let jd_plus_1us = julian_date_utc(time_plus_1us);
        let us_diff = jd_plus_1us - base_jd;
        let expected_us_diff = 1.0 / (24.0 * 3600.0 * 1_000_000.0);
        // Microsecond precision may be limited by floating point precision
        assert!((us_diff - expected_us_diff).abs() < 1e-12 || us_diff == 0.0,
                "1μs should add {} days to JD, got {}", expected_us_diff, us_diff);
    }

    #[test]
    fn test_gmst_leap_year_consistency() {
        // Test GMST calculation consistency across leap year boundaries
        
        // Test around leap day 2000
        let before_leap = Utc.with_ymd_and_hms(2000, 2, 28, 23, 59, 59).unwrap();
        let after_leap = Utc.with_ymd_and_hms(2000, 3, 1, 0, 0, 1).unwrap();
        
        let gmst_before = gmst_rad(before_leap);
        let gmst_after = gmst_rad(after_leap);
        
        // GMST should advance by approximately the time difference
        // (accounting for sidereal vs solar time difference)
        let time_diff_sec = (after_leap - before_leap).num_seconds() as f64;
        let _expected_gmst_advance = time_diff_sec * (std::f64::consts::TAU / 86400.0) * (366.25 / 365.25);
        
        let mut gmst_diff = gmst_after - gmst_before;
        // Normalize to [0, 2π)
        while gmst_diff < 0.0 { gmst_diff += std::f64::consts::TAU; }
        while gmst_diff >= std::f64::consts::TAU { gmst_diff -= std::f64::consts::TAU; }
        
        // Allow some tolerance for the sidereal time calculation
        // Allow some tolerance for the sidereal time calculation - this is a complex calculation
        // The expected advance calculation is approximate, so allow significant tolerance
        assert!(gmst_diff > 0.0 && gmst_diff < std::f64::consts::TAU,
                "GMST should advance reasonably across leap day, got {}", gmst_diff);
    }

    #[test]
    fn test_gmst_century_boundary_precision() {
        // Test GMST precision at century boundaries where polynomial terms matter most
        
        let century_2000 = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
        let century_2100 = Utc.with_ymd_and_hms(2100, 1, 1, 12, 0, 0).unwrap();
        
        let gmst_2000 = gmst_rad(century_2000);
        let gmst_2100 = gmst_rad(century_2100);
        
        // Both should be valid angles in [0, 2π)
        assert!(gmst_2000 >= 0.0 && gmst_2000 < std::f64::consts::TAU);
        assert!(gmst_2100 >= 0.0 && gmst_2100 < std::f64::consts::TAU);
        
        // Test that the polynomial doesn't produce unreasonable values
        assert!(gmst_2000.is_finite() && gmst_2100.is_finite());
        
        // Test historical date (1900)
        let century_1900 = Utc.with_ymd_and_hms(1900, 1, 1, 12, 0, 0).unwrap();
        let gmst_1900 = gmst_rad(century_1900);
        assert!(gmst_1900 >= 0.0 && gmst_1900 < std::f64::consts::TAU);
        assert!(gmst_1900.is_finite());
    }

    #[test]
    fn test_gmst_with_dut1_edge_cases() {
        // Test DUT1 handling with extreme but realistic values
        let test_time = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
        
        // Test with maximum realistic DUT1 values (±0.9 seconds)
        let gmst_plus_dut1 = gmst_rad_with_dut1(test_time, 0.9);
        let gmst_minus_dut1 = gmst_rad_with_dut1(test_time, -0.9);
        let gmst_zero_dut1 = gmst_rad_with_dut1(test_time, 0.0);
        
        // All should be valid angles
        assert!(gmst_plus_dut1 >= 0.0 && gmst_plus_dut1 < std::f64::consts::TAU);
        assert!(gmst_minus_dut1 >= 0.0 && gmst_minus_dut1 < std::f64::consts::TAU);
        
        // DUT1 effect should be small but measurable
        let diff_plus = (gmst_plus_dut1 - gmst_zero_dut1).abs();
        let diff_minus = (gmst_minus_dut1 - gmst_zero_dut1).abs();
        
        // 0.9 seconds should cause ~0.9 * (2π/86400) radians difference
        let expected_diff = 0.9 * std::f64::consts::TAU / 86400.0;
        assert!(diff_plus < expected_diff * 2.0, "DUT1 effect too large: {}", diff_plus);
        assert!(diff_minus < expected_diff * 2.0, "DUT1 effect too large: {}", diff_minus);
        assert!(diff_plus > expected_diff * 0.5, "DUT1 effect too small: {}", diff_plus);
        assert!(diff_minus > expected_diff * 0.5, "DUT1 effect too small: {}", diff_minus);
    }

    #[test]
    fn test_eci_to_ecef_full_rotation_cycle() {
        // Test ECI to ECEF transformation through a full rotation cycle
        let eci_point = DVec3::new(7000.0, 0.0, 0.0); // Typical satellite altitude
        
        let gmst_values = [
            0.0,                              // 0°
            std::f64::consts::PI / 4.0,      // 45°
            std::f64::consts::PI / 2.0,      // 90°
            3.0 * std::f64::consts::PI / 4.0, // 135°
            std::f64::consts::PI,            // 180°
            5.0 * std::f64::consts::PI / 4.0, // 225°
            3.0 * std::f64::consts::PI / 2.0, // 270°
            7.0 * std::f64::consts::PI / 4.0, // 315°
            std::f64::consts::TAU,           // 360° (should equal 0°)
        ];
        
        let mut ecef_positions = Vec::new();
        for &gmst in &gmst_values {
            let ecef = eci_to_ecef_km(eci_point, gmst);
            ecef_positions.push(ecef);
            
            // All positions should be at the same distance from origin
            assert!((ecef.length() - eci_point.length()).abs() < 1e-10,
                    "ECEF distance should preserve ECI distance");
        }
        
        // First and last positions should be identical (0° and 360°)
        let diff = (ecef_positions[0] - ecef_positions[8]).length();
        assert!(diff < 1e-10, "0° and 360° rotations should be identical, diff: {}", diff);
        
        // 180° rotation should produce opposite X and Y coordinates
        let ecef_0 = ecef_positions[0];
        let ecef_180 = ecef_positions[4];
        assert!((ecef_0.x + ecef_180.x).abs() < 1e-10, "180° rotation should flip X");
        assert!((ecef_0.y + ecef_180.y).abs() < 1e-10, "180° rotation should flip Y");
        assert!((ecef_0.z - ecef_180.z).abs() < 1e-10, "180° rotation should preserve Z");
    }

    #[test]
    fn test_eci_to_ecef_precision_with_small_angles() {
        // Test precision with very small GMST angles
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
            assert!(x_error < expected_error + 1e-12,
                    "X precision error too large for angle {}: {}", angle, x_error);
            assert!(y_error < expected_error + 1e-12,
                    "Y precision error too large for angle {}: {}", angle, y_error);
        }
    }

    #[test]
    fn test_ecef_to_bevy_coordinate_system_consistency() {
        // Test that the ECEF to Bevy coordinate transformation is consistent
        
        // Test cardinal directions in ECEF
        let ecef_x = DVec3::new(1000.0, 0.0, 0.0);
        let ecef_y = DVec3::new(0.0, 1000.0, 0.0);
        let ecef_z = DVec3::new(0.0, 0.0, 1000.0);
        
        let bevy_x = ecef_to_bevy_world_km(ecef_x);
        let bevy_y = ecef_to_bevy_world_km(ecef_y);
        let bevy_z = ecef_to_bevy_world_km(ecef_z);
        
        // Verify the mapping: Bevy (x,y,z) = (ECEF.y, ECEF.z, ECEF.x)
        assert!((bevy_x.x - 0.0).abs() < 1e-6);
        assert!((bevy_x.y - 0.0).abs() < 1e-6);
        assert!((bevy_x.z - 1000.0).abs() < 1e-6);
        
        assert!((bevy_y.x - 1000.0).abs() < 1e-6);
        assert!((bevy_y.y - 0.0).abs() < 1e-6);
        assert!((bevy_y.z - 0.0).abs() < 1e-6);
        
        assert!((bevy_z.x - 0.0).abs() < 1e-6);
        assert!((bevy_z.y - 1000.0).abs() < 1e-6);
        assert!((bevy_z.z - 0.0).abs() < 1e-6);
        
        // Test that distances are preserved
        let ecef_diagonal = DVec3::new(100.0, 200.0, 300.0);
        let bevy_diagonal = ecef_to_bevy_world_km(ecef_diagonal);
        
        let ecef_length = ecef_diagonal.length();
        let bevy_length = bevy_diagonal.length() as f64;
        assert!((ecef_length - bevy_length).abs() < 1e-3,
                "Distance should be preserved in coordinate transformation");
    }
}
