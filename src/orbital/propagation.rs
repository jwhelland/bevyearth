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

    // Additional edge case tests for Phase 2
    #[test]
    fn test_minutes_since_epoch_leap_year_boundaries() {
        // Test across leap year boundaries
        let leap_epoch = Utc.with_ymd_and_hms(2000, 2, 28, 12, 0, 0).unwrap();
        let after_leap = Utc.with_ymd_and_hms(2000, 3, 1, 12, 0, 0).unwrap();

        let minutes = minutes_since_epoch(after_leap, leap_epoch);

        // Should be exactly 2 days = 2880 minutes (Feb 28 -> Feb 29 -> Mar 1)
        let expected_minutes = 2.0 * 24.0 * 60.0;
        assert!(
            (minutes - expected_minutes).abs() < 1e-10,
            "Leap year boundary: expected {} minutes, got {}",
            expected_minutes,
            minutes
        );

        // Test non-leap year (1900)
        let non_leap_epoch = Utc.with_ymd_and_hms(1900, 2, 28, 12, 0, 0).unwrap();
        let after_non_leap = Utc.with_ymd_and_hms(1900, 3, 1, 12, 0, 0).unwrap();

        let non_leap_minutes = minutes_since_epoch(after_non_leap, non_leap_epoch);

        // Should be exactly 1 day = 1440 minutes (Feb 28 -> Mar 1, no Feb 29)
        let expected_non_leap = 1.0 * 24.0 * 60.0;
        assert!(
            (non_leap_minutes - expected_non_leap).abs() < 1e-10,
            "Non-leap year: expected {} minutes, got {}",
            expected_non_leap,
            non_leap_minutes
        );
    }

    #[test]
    fn test_minutes_since_epoch_century_boundaries() {
        // Test across century boundaries
        let century_1900 = Utc.with_ymd_and_hms(1900, 1, 1, 0, 0, 0).unwrap();
        let century_2000 = Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();

        let minutes_century = minutes_since_epoch(century_2000, century_1900);

        // 100 years with 24 leap years (not 25, because 1900 wasn't a leap year)
        // = 36524 days = 52,594,560 minutes
        let expected_century_minutes = 36524.0 * 24.0 * 60.0;
        assert!(
            (minutes_century - expected_century_minutes).abs() < 1.0,
            "Century span: expected {} minutes, got {}",
            expected_century_minutes,
            minutes_century
        );
    }

    #[test]
    fn test_minutes_since_epoch_high_precision() {
        // Test high precision with nanoseconds
        let epoch = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

        // Add exactly 1.5 minutes with nanosecond precision
        let sim_time =
            epoch + chrono::Duration::seconds(90) + chrono::Duration::nanoseconds(500_000_000);

        let minutes = minutes_since_epoch(sim_time, epoch);
        let expected = 1.5 + (500_000_000.0 / 1e9) / 60.0; // 1.5 + 0.5s in minutes

        assert!(
            (minutes - expected).abs() < 1e-12,
            "High precision: expected {} minutes, got {}",
            expected,
            minutes
        );

        // Test with microsecond precision
        let micro_time = epoch + chrono::Duration::microseconds(1);
        let micro_minutes = minutes_since_epoch(micro_time, epoch);
        let expected_micro = 1.0 / (60.0 * 1_000_000.0); // 1 microsecond in minutes

        assert!(
            (micro_minutes - expected_micro).abs() < 1e-15,
            "Microsecond precision: expected {} minutes, got {}",
            expected_micro,
            micro_minutes
        );
    }

    #[test]
    fn test_minutes_since_epoch_large_time_spans() {
        // Test with very large time spans (decades)
        let epoch_1970 = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap();
        let time_2024 = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

        let minutes_54_years = minutes_since_epoch(time_2024, epoch_1970);

        // 54 years from 1970 to 2024
        // Approximate: 54 * 365.25 * 24 * 60 minutes
        let approx_minutes = 54.0 * 365.25 * 24.0 * 60.0;
        let diff = (minutes_54_years - approx_minutes).abs();

        // Should be within a few days worth of minutes
        assert!(
            diff < 10.0 * 24.0 * 60.0,
            "54-year span should be approximately {} minutes, got {} (diff: {})",
            approx_minutes,
            minutes_54_years,
            diff
        );

        // Test future dates
        let future_2100 = Utc.with_ymd_and_hms(2100, 1, 1, 0, 0, 0).unwrap();
        let future_minutes = minutes_since_epoch(future_2100, time_2024);

        // 76 years from 2024 to 2100
        let approx_future = 76.0 * 365.25 * 24.0 * 60.0;
        let future_diff = (future_minutes - approx_future).abs();

        assert!(
            future_diff < 10.0 * 24.0 * 60.0,
            "Future 76-year span should be approximately {} minutes, got {}",
            approx_future,
            future_minutes
        );
    }

    #[test]
    fn test_minutes_since_epoch_negative_spans() {
        // Test when sim_time is before epoch (negative result)
        let epoch = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
        let before_epoch = Utc.with_ymd_and_hms(2024, 6, 15, 10, 0, 0).unwrap();

        let negative_minutes = minutes_since_epoch(before_epoch, epoch);

        // Should be -120 minutes (2 hours before)
        assert!(
            (negative_minutes + 120.0).abs() < 1e-10,
            "Negative span: expected -120 minutes, got {}",
            negative_minutes
        );

        // Test with fractional negative time
        let before_frac = epoch - chrono::Duration::seconds(90); // 1.5 minutes before
        let negative_frac = minutes_since_epoch(before_frac, epoch);

        assert!(
            (negative_frac + 1.5).abs() < 1e-10,
            "Negative fractional: expected -1.5 minutes, got {}",
            negative_frac
        );
    }

    #[test]
    fn test_minutes_since_epoch_same_time() {
        // Test when sim_time equals epoch
        let epoch = Utc.with_ymd_and_hms(2024, 6, 15, 12, 30, 45).unwrap();
        let same_time = epoch;

        let zero_minutes = minutes_since_epoch(same_time, epoch);

        assert!(
            zero_minutes.abs() < 1e-15,
            "Same time should give 0 minutes, got {}",
            zero_minutes
        );
    }

    #[test]
    fn test_minutes_since_epoch_daylight_saving_transitions() {
        // Test around daylight saving time transitions (though UTC shouldn't be affected)
        // This tests the robustness of the calculation with times that might be problematic in local time

        // Spring forward in 2024 (second Sunday in March for US)
        let before_spring = Utc.with_ymd_and_hms(2024, 3, 10, 6, 0, 0).unwrap(); // 1 AM EST
        let after_spring = Utc.with_ymd_and_hms(2024, 3, 10, 8, 0, 0).unwrap(); // 3 AM EST (after spring forward)

        let spring_minutes = minutes_since_epoch(after_spring, before_spring);

        // Should be exactly 2 hours = 120 minutes (UTC is not affected by DST)
        assert!(
            (spring_minutes - 120.0).abs() < 1e-10,
            "Spring DST transition: expected 120 minutes, got {}",
            spring_minutes
        );

        // Fall back in 2024 (first Sunday in November for US)
        let before_fall = Utc.with_ymd_and_hms(2024, 11, 3, 5, 0, 0).unwrap(); // 1 AM EST
        let after_fall = Utc.with_ymd_and_hms(2024, 11, 3, 7, 0, 0).unwrap(); // 1 AM EST (after fall back)

        let fall_minutes = minutes_since_epoch(after_fall, before_fall);

        // Should be exactly 2 hours = 120 minutes (UTC is not affected by DST)
        assert!(
            (fall_minutes - 120.0).abs() < 1e-10,
            "Fall DST transition: expected 120 minutes, got {}",
            fall_minutes
        );
    }

    #[test]
    fn test_minutes_since_epoch_extreme_precision() {
        // Test with the smallest possible time differences
        let epoch = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();

        // Add 1 nanosecond
        let nano_time = epoch + chrono::Duration::nanoseconds(1);
        let nano_minutes = minutes_since_epoch(nano_time, epoch);

        let expected_nano_minutes = 1.0 / (60.0 * 1_000_000_000.0);
        assert!(
            (nano_minutes - expected_nano_minutes).abs() < 1e-18,
            "Nanosecond precision: expected {} minutes, got {}",
            expected_nano_minutes,
            nano_minutes
        );

        // Test with maximum nanosecond value within a second
        let max_nano_time = epoch + chrono::Duration::nanoseconds(999_999_999);
        let max_nano_minutes = minutes_since_epoch(max_nano_time, epoch);

        let expected_max_nano = 999_999_999.0 / (60.0 * 1_000_000_000.0);
        assert!(
            (max_nano_minutes - expected_max_nano).abs() < 1e-15,
            "Max nanosecond precision: expected {} minutes, got {}",
            expected_max_nano,
            max_nano_minutes
        );
    }

    #[test]
    fn test_minutes_since_epoch_consistency_with_duration() {
        // Test that our calculation is consistent with chrono::Duration
        let epoch = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let sim_time = Utc.with_ymd_and_hms(2024, 1, 2, 3, 45, 30).unwrap();

        let our_minutes = minutes_since_epoch(sim_time, epoch);

        // Calculate using chrono::Duration directly
        let duration = sim_time - epoch;
        let duration_minutes =
            duration.num_seconds() as f64 / 60.0 + (duration.subsec_nanos() as f64) / (60.0 * 1e9);

        assert!(
            (our_minutes - duration_minutes).abs() < 1e-12,
            "Should match chrono::Duration calculation: our {} vs duration {}",
            our_minutes,
            duration_minutes
        );
    }
}
