//! Orbital propagation utilities

use chrono::{DateTime, Utc};

/// Calculate minutes since epoch for SGP4 propagation
pub fn minutes_since_epoch(sim_utc: DateTime<Utc>, epoch: DateTime<Utc>) -> f64 {
    let delta = sim_utc - epoch;
    delta.num_seconds() as f64 / 60.0 + (delta.subsec_nanos() as f64) / 60.0 / 1.0e9
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_minutes_since_epoch() {
        let epoch = Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
        let sim_time = Utc.with_ymd_and_hms(2000, 1, 1, 1, 0, 0).unwrap();

        let minutes = minutes_since_epoch(sim_time, epoch);
        assert!((minutes - 60.0).abs() < 1e-10);

        // Test with fractional seconds
        let sim_time_frac = Utc.with_ymd_and_hms(2000, 1, 1, 0, 1, 30).unwrap();
        let minutes_frac = minutes_since_epoch(sim_time_frac, epoch);
        assert!((minutes_frac - 1.5).abs() < 1e-6);
    }

    #[test]
    fn test_minutes_since_epoch_negative() {
        let epoch = Utc.with_ymd_and_hms(2000, 1, 1, 1, 0, 0).unwrap();
        let sim_time = Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();

        let minutes = minutes_since_epoch(sim_time, epoch);
        assert!((minutes + 60.0).abs() < 1e-10);
    }
}
