//! TLE parsing utilities

use chrono::{DateTime, Utc};

/// Parse TLE epoch from line 1 to UTC DateTime
pub fn parse_tle_epoch_to_utc(line1: &str) -> Option<DateTime<Utc>> {
    // TLE line1 epoch fields (columns 19–32, 1-based; 18..32 0-based)
    if line1.len() < 32 {
        return None;
    }
    let s = &line1[18..32];
    let mut parts = s.trim().split('.');
    let yyddd = parts.next()?;
    let frac = parts.next().unwrap_or("0");
    if yyddd.len() < 3 {
        return None;
    }
    let (yy_str, ddd_str) = yyddd.split_at(2);
    let yy: i32 = yy_str.parse().ok()?;
    let ddd: i32 = ddd_str.parse().ok()?;
    let year = if yy >= 57 { 1900 + yy } else { 2000 + yy };
    let jan1 = chrono::NaiveDate::from_ymd_opt(year, 1, 1)?;
    let date = jan1.checked_add_signed(chrono::Duration::days((ddd - 1) as i64))?;
    let frac_sec: f64 = match format!("0.{}", frac).parse::<f64>() {
        Ok(v) => v * 86400.0,
        Err(_) => return None,
    };
    let secs = frac_sec.trunc() as i64;
    let nanos = ((frac_sec - (secs as f64)) * 1e9).round() as i64;
    let dt = date.and_hms_opt(0, 0, 0)?;
    let mut ndt = chrono::NaiveDateTime::new(date, dt.time());
    ndt = ndt + chrono::Duration::seconds(secs);
    ndt = ndt + chrono::Duration::nanoseconds(nanos);
    Some(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tle_epoch() {
        // Test with a typical TLE line 1
        let line1 = "1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927";
        let result = parse_tle_epoch_to_utc(line1);
        assert!(result.is_some());
        
        // Test with invalid line
        let invalid_line = "too short";
        let result = parse_tle_epoch_to_utc(invalid_line);
        assert!(result.is_none());
    }
}