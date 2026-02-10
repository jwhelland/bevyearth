//! Low-precision Moon ephemeris and state updates.

use bevy::math::DVec3;
use bevy::prelude::*;
use chrono::{DateTime, Utc};

use crate::core::coordinates::{eci_to_ecef_km, gmst_rad_with_dut1, julian_date_utc};
use crate::orbital::{Dut1, SimulationTime};

/// Canonical Moon position in ECEF (km).
#[derive(Resource, Deref, DerefMut, Copy, Clone, Debug)]
pub struct MoonEcefKm(pub DVec3);

impl Default for MoonEcefKm {
    fn default() -> Self {
        Self(DVec3::ZERO)
    }
}

#[derive(Copy, Clone)]
struct LonDistTerm {
    d: i32,
    m: i32,
    mp: i32,
    f: i32,
    l: i32,
    r: i32,
}

#[derive(Copy, Clone)]
struct LatTerm {
    d: i32,
    m: i32,
    mp: i32,
    f: i32,
    b: i32,
}

// Dominant terms from Meeus Table 45.A (longitude + distance).
// Coefficients: l in 1e-6 degrees, r in 1e-3 km.
const LON_DIST_TERMS: [LonDistTerm; 20] = [
    LonDistTerm {
        d: 0,
        m: 0,
        mp: 1,
        f: 0,
        l: 6288774,
        r: -20905355,
    },
    LonDistTerm {
        d: 2,
        m: 0,
        mp: -1,
        f: 0,
        l: 1274027,
        r: -3699111,
    },
    LonDistTerm {
        d: 2,
        m: 0,
        mp: 0,
        f: 0,
        l: 658314,
        r: -2955968,
    },
    LonDistTerm {
        d: 0,
        m: 0,
        mp: 2,
        f: 0,
        l: 213618,
        r: -569925,
    },
    LonDistTerm {
        d: 0,
        m: 1,
        mp: 0,
        f: 0,
        l: -185116,
        r: 48888,
    },
    LonDistTerm {
        d: 0,
        m: 0,
        mp: 0,
        f: 2,
        l: -114332,
        r: -3149,
    },
    LonDistTerm {
        d: 2,
        m: 0,
        mp: -2,
        f: 0,
        l: 58793,
        r: 246158,
    },
    LonDistTerm {
        d: 2,
        m: -1,
        mp: -1,
        f: 0,
        l: 57066,
        r: -152138,
    },
    LonDistTerm {
        d: 2,
        m: 0,
        mp: 1,
        f: 0,
        l: 53322,
        r: -170733,
    },
    LonDistTerm {
        d: 2,
        m: -1,
        mp: 0,
        f: 0,
        l: 45758,
        r: -204586,
    },
    LonDistTerm {
        d: 0,
        m: 1,
        mp: -1,
        f: 0,
        l: -40923,
        r: -129620,
    },
    LonDistTerm {
        d: 1,
        m: 0,
        mp: 0,
        f: 0,
        l: -34720,
        r: 108743,
    },
    LonDistTerm {
        d: 0,
        m: 1,
        mp: 1,
        f: 0,
        l: -30383,
        r: 104755,
    },
    LonDistTerm {
        d: 2,
        m: 0,
        mp: 0,
        f: -2,
        l: 15327,
        r: 10321,
    },
    LonDistTerm {
        d: 0,
        m: 0,
        mp: 1,
        f: 2,
        l: -12528,
        r: 0,
    },
    LonDistTerm {
        d: 0,
        m: 0,
        mp: 1,
        f: -2,
        l: 10980,
        r: 79661,
    },
    LonDistTerm {
        d: 4,
        m: 0,
        mp: -1,
        f: 0,
        l: 10675,
        r: -34782,
    },
    LonDistTerm {
        d: 0,
        m: 0,
        mp: 3,
        f: 0,
        l: 10034,
        r: -23210,
    },
    LonDistTerm {
        d: 4,
        m: 0,
        mp: -2,
        f: 0,
        l: 8548,
        r: -21636,
    },
    LonDistTerm {
        d: 2,
        m: 1,
        mp: -1,
        f: 0,
        l: -7888,
        r: 24208,
    },
];

// Dominant terms from Meeus Table 45.B (latitude).
// Coefficients: b in 1e-6 degrees.
const LAT_TERMS: [LatTerm; 20] = [
    LatTerm {
        d: 0,
        m: 0,
        mp: 0,
        f: 1,
        b: 5128122,
    },
    LatTerm {
        d: 0,
        m: 0,
        mp: 1,
        f: 1,
        b: 280602,
    },
    LatTerm {
        d: 0,
        m: 0,
        mp: 1,
        f: -1,
        b: 277693,
    },
    LatTerm {
        d: 2,
        m: 0,
        mp: 0,
        f: -1,
        b: 173237,
    },
    LatTerm {
        d: 2,
        m: 0,
        mp: -1,
        f: 1,
        b: 55413,
    },
    LatTerm {
        d: 2,
        m: 0,
        mp: -1,
        f: -1,
        b: 46271,
    },
    LatTerm {
        d: 2,
        m: 0,
        mp: 0,
        f: 1,
        b: 32573,
    },
    LatTerm {
        d: 0,
        m: 0,
        mp: 2,
        f: 1,
        b: 17198,
    },
    LatTerm {
        d: 2,
        m: 0,
        mp: 1,
        f: -1,
        b: 9266,
    },
    LatTerm {
        d: 0,
        m: 0,
        mp: 2,
        f: -1,
        b: 8822,
    },
    LatTerm {
        d: 2,
        m: -1,
        mp: 0,
        f: -1,
        b: 8216,
    },
    LatTerm {
        d: 2,
        m: 0,
        mp: -2,
        f: -1,
        b: 4324,
    },
    LatTerm {
        d: 2,
        m: 0,
        mp: 1,
        f: 1,
        b: 4200,
    },
    LatTerm {
        d: 2,
        m: 1,
        mp: 0,
        f: -1,
        b: -3359,
    },
    LatTerm {
        d: 2,
        m: -1,
        mp: -1,
        f: 1,
        b: 2463,
    },
    LatTerm {
        d: 2,
        m: -1,
        mp: 0,
        f: 1,
        b: 2211,
    },
    LatTerm {
        d: 2,
        m: -1,
        mp: -1,
        f: -1,
        b: 2065,
    },
    LatTerm {
        d: 0,
        m: 1,
        mp: -1,
        f: -1,
        b: -1870,
    },
    LatTerm {
        d: 4,
        m: 0,
        mp: -1,
        f: -1,
        b: 1828,
    },
    LatTerm {
        d: 0,
        m: 1,
        mp: 0,
        f: 1,
        b: -1794,
    },
];

fn normalize_deg(deg: f64) -> f64 {
    deg.rem_euclid(360.0)
}

/// Approximate Moon position in ECEF (km) using low-precision Meeus terms.
pub fn moon_position_ecef_km(utc: DateTime<Utc>, dut1_seconds: f64) -> DVec3 {
    let jd = julian_date_utc(utc);
    let t = (jd - 2451545.0) / 36525.0;

    let l_prime = normalize_deg(
        218.3164477 + 481267.88123421 * t - 0.0015786 * t * t + t * t * t / 538841.0
            - t * t * t * t / 65194000.0,
    );
    let d = normalize_deg(
        297.8501921 + 445267.1114034 * t - 0.0018819 * t * t + t * t * t / 545868.0
            - t * t * t * t / 113065000.0,
    );
    let m =
        normalize_deg(357.5291092 + 35999.0502909 * t - 0.0001536 * t * t + t * t * t / 24490000.0);
    let mp = normalize_deg(
        134.9633964 + 477198.8675055 * t + 0.0087414 * t * t + t * t * t / 69699.0
            - t * t * t * t / 14712000.0,
    );
    let f = normalize_deg(
        93.2720950 + 483202.0175233 * t - 0.0036539 * t * t - t * t * t / 3526000.0
            + t * t * t * t / 863310000.0,
    );

    let e = 1.0 - 0.002516 * t - 0.0000074 * t * t;

    let mut sum_l = 0.0;
    let mut sum_r = 0.0;
    let mut sum_b = 0.0;

    for term in LON_DIST_TERMS {
        let arg_deg =
            term.d as f64 * d + term.m as f64 * m + term.mp as f64 * mp + term.f as f64 * f;
        let arg = arg_deg.to_radians();
        let e_factor = match term.m.abs() {
            1 => e,
            2 => e * e,
            _ => 1.0,
        };
        sum_l += term.l as f64 * e_factor * arg.sin();
        sum_r += term.r as f64 * e_factor * arg.cos();
    }

    for term in LAT_TERMS {
        let arg_deg =
            term.d as f64 * d + term.m as f64 * m + term.mp as f64 * mp + term.f as f64 * f;
        let arg = arg_deg.to_radians();
        let e_factor = match term.m.abs() {
            1 => e,
            2 => e * e,
            _ => 1.0,
        };
        sum_b += term.b as f64 * e_factor * arg.sin();
    }

    let l_prime_rad = l_prime.to_radians();
    let f_rad = f.to_radians();
    let mp_rad = mp.to_radians();

    let a1 = (119.75 + 131.849 * t).to_radians();
    let a2 = (53.09 + 479264.290 * t).to_radians();
    let a3 = (313.45 + 481266.484 * t).to_radians();

    sum_l += 3958.0 * a1.sin() + 1962.0 * (l_prime_rad - f_rad).sin() + 318.0 * a2.sin();
    sum_b += -2235.0 * l_prime_rad.sin()
        + 382.0 * a3.sin()
        + 175.0 * (a1 - f_rad).sin()
        + 175.0 * (a1 + f_rad).sin()
        + 127.0 * (l_prime_rad - mp_rad).sin()
        - 115.0 * (l_prime_rad + mp_rad).sin();

    let lambda = (l_prime + sum_l / 1_000_000.0).to_radians();
    let beta = (sum_b / 1_000_000.0).to_radians();
    let delta_km = 385000.56 + sum_r / 1000.0;

    let x = delta_km * beta.cos() * lambda.cos();
    let y = delta_km * beta.cos() * lambda.sin();
    let z = delta_km * beta.sin();

    let eps = (23.439291 - 0.0130042 * t).to_radians();
    let y_eq = y * eps.cos() - z * eps.sin();
    let z_eq = y * eps.sin() + z * eps.cos();

    let eci = DVec3::new(x, y_eq, z_eq);
    let gmst = gmst_rad_with_dut1(utc, dut1_seconds);
    eci_to_ecef_km(eci, gmst)
}

/// Update Moon position from the current simulation time.
pub fn update_moon_state(
    sim_time: Res<SimulationTime>,
    dut1: Res<Dut1>,
    mut moon: ResMut<MoonEcefKm>,
) {
    if !sim_time.is_changed() && !dut1.is_changed() {
        return;
    }
    moon.0 = moon_position_ecef_km(sim_time.current_utc, **dut1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_moon_distance_bounds() {
        let times = [
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
        ];
        for t in times {
            let ecef = moon_position_ecef_km(t, 0.0);
            let dist = ecef.length();
            assert!(
                (350_000.0..=450_000.0).contains(&dist),
                "distance out of bounds: {}",
                dist
            );
        }
    }

    #[test]
    fn test_moon_position_finite() {
        let t = Utc.with_ymd_and_hms(2024, 3, 20, 12, 0, 0).unwrap();
        let ecef = moon_position_ecef_km(t, 0.0);
        assert!(ecef.x.is_finite());
        assert!(ecef.y.is_finite());
        assert!(ecef.z.is_finite());
    }
}
