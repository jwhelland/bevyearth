//! Time management for orbital mechanics

use bevy::prelude::*;
use chrono::{DateTime, Duration, Utc};

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
        sim_time.current_utc = sim_time.current_utc + Duration::seconds(whole);
    }
    if nanos != 0 {
        sim_time.current_utc = sim_time.current_utc + Duration::nanoseconds(nanos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_time_default() {
        let sim_time = SimulationTime::default();
        assert_eq!(sim_time.time_scale, 1.0);
        // Just check that current_utc is set to something reasonable
        assert!(sim_time.current_utc.timestamp() > 0);
    }
}