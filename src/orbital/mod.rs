//! Orbital mechanics module
//!
//! This module handles orbital calculations, coordinate transformations,
//! and time management for satellite propagation.

use bevy::prelude::*;

pub mod coordinates;
pub mod propagation;
pub mod time;

pub use coordinates::{eci_to_ecef_km, ecef_to_bevy_world_km, gmst_rad_with_dut1};
pub use propagation::minutes_since_epoch;
pub use time::{SimulationTime, advance_simulation_clock, Dut1};

/// Plugin for orbital mechanics and time management
pub struct OrbitalPlugin;

impl Plugin for OrbitalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationTime>()
            .init_resource::<Dut1>()
            .add_systems(Update, advance_simulation_clock);
    }
}
