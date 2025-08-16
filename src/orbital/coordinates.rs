//! Coordinate transformation utilities for orbital mechanics

use bevy::math::DVec3;
use chrono::{DateTime, Utc};

/// Approximate GMST (Greenwich Mean Sidereal Time) for visualization
pub fn gmst_rad(t: DateTime<Utc>) -> f64 {
    let secs = t.timestamp() as f64 + (t.timestamp_subsec_nanos() as f64) * 1e-9;
    let omega = std::f64::consts::TAU / 86164.0905_f64;
    (secs * omega).rem_euclid(std::f64::consts::TAU)
}

/// Rotate ECI (TEME) -> ECEF using simple GMST rotation about Z
pub fn eci_to_ecef_km(eci: DVec3, gmst: f64) -> DVec3 {
    let (s, c) = gmst.sin_cos();
    let x = c * eci.x - s * eci.y;
    let y = s * eci.x + c * eci.y;
    DVec3::new(x, y, eci.z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_gmst_rad() {
        // Test with a known time
        let test_time = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
        let gmst = gmst_rad(test_time);

        // GMST should be between 0 and 2π
        assert!(gmst >= 0.0);
        assert!(gmst < std::f64::consts::TAU);
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

        // Test with 90 degree rotation
        let gmst_90 = std::f64::consts::PI / 2.0;
        let ecef_90 = eci_to_ecef_km(eci, gmst_90);

        // X should become -Y, Y should become X
        assert!(ecef_90.x.abs() < 1e-10);
        assert!((ecef_90.y - 1000.0).abs() < 1e-10);
        assert!(ecef_90.z.abs() < 1e-10);
    }
}
