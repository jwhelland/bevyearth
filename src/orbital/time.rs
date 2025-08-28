//! Time management for orbital mechanics

use bevy::prelude::*;
use chrono::{DateTime, Duration, Utc};

#[cfg(test)]
use chrono::{Datelike, TimeZone, Timelike};

/// Simulation time resource
#[derive(Resource)]
pub struct SimulationTime {
    pub current_utc: DateTime<Utc>,
    pub time_scale: f32,
}

impl Default for SimulationTime {
    fn default() -> Self {
        Self {
            current_utc: Utc::now(),
            time_scale: 1.0,
        }
    }
}

/// System to advance simulation UTC by scale
pub fn advance_simulation_clock(time: Res<Time>, mut sim_time: ResMut<SimulationTime>) {
    let scaled = (time.delta_secs() * sim_time.time_scale).max(0.0);
    let whole = scaled.trunc() as i64;
    let nanos = ((scaled - scaled.trunc()) * 1_000_000_000.0) as i64;
    if whole != 0 {
        sim_time.current_utc += Duration::seconds(whole);
    }
    if nanos != 0 {
        sim_time.current_utc += Duration::nanoseconds(nanos);
    }
}

/// Resource for UT1-UTC (DUT1) seconds used in GMST computation.
/// Defaults to 0.0 which is acceptable for visualization.
#[derive(Resource, Deref, DerefMut)]
pub struct Dut1(pub f64);

impl Default for Dut1 {
    fn default() -> Self {
        Self(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function for testing time advancement without Bevy resources
    fn test_advance_time(sim_time: &mut SimulationTime, delta_seconds: f32) {
        let scaled = (delta_seconds * sim_time.time_scale).max(0.0);
        let whole = scaled.trunc() as i64;
        let nanos = ((scaled - scaled.trunc()) * 1_000_000_000.0) as i64;
        if whole != 0 {
            sim_time.current_utc += Duration::seconds(whole);
        }
        if nanos != 0 {
            sim_time.current_utc += Duration::nanoseconds(nanos);
        }
    }

    #[test]
    fn test_simulation_time_default() {
        let sim_time = SimulationTime::default();
        assert_eq!(sim_time.time_scale, 1.0);
        // Just check that current_utc is set to something reasonable
        assert!(sim_time.current_utc.timestamp() > 0);
    }

    // Additional edge case tests for Phase 2
    #[test]
    fn test_advance_simulation_clock_with_leap_seconds() {
        // Test time advancement across potential leap second boundaries
        let mut sim_time = SimulationTime {
            current_utc: Utc.with_ymd_and_hms(2016, 12, 31, 23, 59, 59).unwrap(),
            time_scale: 1.0,
        };

        // Simulate advancing by 2 seconds (crossing into new year)
        test_advance_time(&mut sim_time, 2.0);

        // Should advance to 2017-01-01 00:00:01
        assert_eq!(sim_time.current_utc.year(), 2017);
        assert_eq!(sim_time.current_utc.month(), 1);
        assert_eq!(sim_time.current_utc.day(), 1);
        assert_eq!(sim_time.current_utc.hour(), 0);
        assert_eq!(sim_time.current_utc.minute(), 0);
        assert_eq!(sim_time.current_utc.second(), 1);
    }

    #[test]
    fn test_advance_simulation_clock_leap_year_boundary() {
        // Test advancing across leap day boundary
        let mut sim_time = SimulationTime {
            current_utc: Utc.with_ymd_and_hms(2000, 2, 28, 23, 59, 58).unwrap(),
            time_scale: 1.0,
        };

        // Advance by 2 seconds to cross into leap day
        test_advance_time(&mut sim_time, 2.0);

        // Should be on Feb 29, 2000
        assert_eq!(sim_time.current_utc.year(), 2000);
        assert_eq!(sim_time.current_utc.month(), 2);
        assert_eq!(sim_time.current_utc.day(), 29);
        assert_eq!(sim_time.current_utc.hour(), 0);
        assert_eq!(sim_time.current_utc.minute(), 0);
        assert_eq!(sim_time.current_utc.second(), 0);

        // Advance by 24 hours to cross into March
        test_advance_time(&mut sim_time, 24.0 * 3600.0);

        // Should be March 1, 2000
        assert_eq!(sim_time.current_utc.year(), 2000);
        assert_eq!(sim_time.current_utc.month(), 3);
        assert_eq!(sim_time.current_utc.day(), 1);
    }

    #[test]
    fn test_advance_simulation_clock_non_leap_year() {
        // Test that non-leap years don't have Feb 29
        let mut sim_time = SimulationTime {
            current_utc: Utc.with_ymd_and_hms(1900, 2, 28, 12, 0, 0).unwrap(),
            time_scale: 1.0,
        };

        // Advance by 12 hours to cross into March (skipping Feb 29)
        test_advance_time(&mut sim_time, 12.0 * 3600.0);

        // Should be March 1, 1900 (1900 was not a leap year)
        assert_eq!(sim_time.current_utc.year(), 1900);
        assert_eq!(sim_time.current_utc.month(), 3);
        assert_eq!(sim_time.current_utc.day(), 1);
    }

    #[test]
    fn test_advance_simulation_clock_with_high_time_scale() {
        // Test time advancement with high time scale factors
        let mut sim_time = SimulationTime {
            current_utc: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            time_scale: 3600.0, // 1 real second = 1 simulated hour
        };

        let original_time = sim_time.current_utc;

        // Advance by 1 real second (should be 1 simulated hour)
        test_advance_time(&mut sim_time, 1.0);

        let time_diff = sim_time.current_utc - original_time;
        assert_eq!(time_diff.num_seconds(), 3600);

        // Test with fractional time scale
        sim_time.time_scale = 86400.0; // 1 real second = 1 simulated day
        let before_day_advance = sim_time.current_utc;

        test_advance_time(&mut sim_time, 1.0);

        let day_diff = sim_time.current_utc - before_day_advance;
        assert_eq!(day_diff.num_seconds(), 86400);
    }

    #[test]
    fn test_advance_simulation_clock_with_fractional_seconds() {
        // Test precise fractional second handling
        let mut sim_time = SimulationTime {
            current_utc: Utc.with_ymd_and_hms(2024, 6, 15, 12, 30, 45).unwrap(),
            time_scale: 1.0,
        };

        let original_time = sim_time.current_utc;

        // Advance by 0.5 seconds
        test_advance_time(&mut sim_time, 0.5);

        let time_diff = sim_time.current_utc - original_time;
        let nanos_diff = time_diff.num_nanoseconds().unwrap();

        // Should be exactly 500,000,000 nanoseconds
        assert_eq!(nanos_diff, 500_000_000);

        // Test with very small fractional seconds
        let before_micro = sim_time.current_utc;
        test_advance_time(&mut sim_time, 0.000001);

        let micro_diff = sim_time.current_utc - before_micro;
        let micro_nanos = micro_diff.num_nanoseconds().unwrap();
        assert_eq!(micro_nanos, 1000); // 1 microsecond = 1000 nanoseconds
    }

    #[test]
    fn test_advance_simulation_clock_negative_time_scale() {
        // Test that negative time scales are handled (clamped to 0)
        let mut sim_time = SimulationTime {
            current_utc: Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap(),
            time_scale: -1.0, // Negative time scale
        };

        let original_time = sim_time.current_utc;

        // Try to advance by 1 second with negative scale
        test_advance_time(&mut sim_time, 1.0);

        // Time should not change (negative scale clamped to 0)
        assert_eq!(sim_time.current_utc, original_time);
    }

    #[test]
    fn test_advance_simulation_clock_zero_time_scale() {
        // Test that zero time scale pauses simulation
        let mut sim_time = SimulationTime {
            current_utc: Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap(),
            time_scale: 0.0,
        };

        let original_time = sim_time.current_utc;

        // Try to advance by 1 second with zero scale
        test_advance_time(&mut sim_time, 1.0);

        // Time should not change
        assert_eq!(sim_time.current_utc, original_time);
    }

    #[test]
    fn test_advance_simulation_clock_extreme_time_scales() {
        // Test with very large time scales
        let mut sim_time = SimulationTime {
            current_utc: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            time_scale: 365.25 * 24.0 * 3600.0, // 1 real second = 1 simulated year
        };

        let original_time = sim_time.current_utc;
        let original_year = sim_time.current_utc.year();

        // Advance by 1 real second (should be ~1 simulated year)
        test_advance_time(&mut sim_time, 1.0);

        // Should advance by approximately 1 year
        let year_diff = sim_time.current_utc.year() - original_year;
        // With such a large time scale, we expect significant advancement
        assert!(
            year_diff >= 0,
            "Should advance time, got {} year difference",
            year_diff
        );

        // Check that time actually advanced significantly
        let time_diff = sim_time.current_utc - original_time;
        let seconds_diff = time_diff.num_seconds();
        assert!(
            seconds_diff > 1000000,
            "Should advance by many seconds, got {}",
            seconds_diff
        );

        // Test with very small time scale
        sim_time.time_scale = 1e-6; // Very slow simulation
        let before_slow = sim_time.current_utc;

        test_advance_time(&mut sim_time, 1.0);

        let slow_diff = sim_time.current_utc - before_slow;
        let slow_nanos = slow_diff.num_nanoseconds().unwrap_or(0);

        // Should advance by ~1 microsecond
        assert!(
            slow_nanos > 0 && slow_nanos < 10000,
            "Slow advance should be small: {} ns",
            slow_nanos
        );
    }

    #[test]
    fn test_dut1_resource_default() {
        let dut1 = Dut1::default();
        assert_eq!(*dut1, 0.0);
    }

    #[test]
    fn test_dut1_resource_deref() {
        let mut dut1 = Dut1(0.5);
        assert_eq!(*dut1, 0.5);

        // Test DerefMut
        *dut1 = -0.3;
        assert_eq!(*dut1, -0.3);
    }

    #[test]
    fn test_simulation_time_consistency_across_boundaries() {
        // Test that simulation time remains consistent across various boundaries
        let test_cases = vec![
            // (start_time, advance_seconds, expected_final_components)
            (
                Utc.with_ymd_and_hms(2023, 12, 31, 23, 59, 59).unwrap(),
                2.0,
                (2024, 1, 1, 0, 0, 1),
            ),
            (
                Utc.with_ymd_and_hms(2024, 2, 28, 23, 59, 59).unwrap(),
                2.0,
                (2024, 2, 29, 0, 0, 1), // 2024 is a leap year
            ),
            (
                Utc.with_ymd_and_hms(2024, 6, 30, 23, 59, 59).unwrap(),
                2.0,
                (2024, 7, 1, 0, 0, 1),
            ),
        ];

        for (start_time, advance_sec, (exp_year, exp_month, exp_day, exp_hour, exp_min, exp_sec)) in
            test_cases
        {
            let mut sim_time = SimulationTime {
                current_utc: start_time,
                time_scale: 1.0,
            };

            test_advance_time(&mut sim_time, advance_sec as f32);

            assert_eq!(sim_time.current_utc.year(), exp_year);
            assert_eq!(sim_time.current_utc.month(), exp_month);
            assert_eq!(sim_time.current_utc.day(), exp_day);
            assert_eq!(sim_time.current_utc.hour(), exp_hour);
            assert_eq!(sim_time.current_utc.minute(), exp_min);
            assert_eq!(sim_time.current_utc.second(), exp_sec);
        }
    }
}
