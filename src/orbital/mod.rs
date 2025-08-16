//! Orbital mechanics module
//!
//! This module handles orbital calculations, coordinate transformations,
//! and time management for satellite propagation.

use bevy::prelude::*;

pub mod coordinates;
pub mod propagation;
pub mod time;

pub use coordinates::{eci_to_ecef_km, gmst_rad};
pub use propagation::minutes_since_epoch;
pub use time::{SimulationTime, advance_simulation_clock};

/// Plugin for orbital mechanics and time management
pub struct OrbitalPlugin;

impl Plugin for OrbitalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationTime>()
            .add_systems(Update, advance_simulation_clock);
    }
}
