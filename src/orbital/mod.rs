//! Orbital mechanics module
//!
//! This module handles orbital calculations, coordinate transformations,
//! and time management for satellite propagation.

use bevy::prelude::*;

pub mod moon;
pub mod propagation;
pub mod time;

pub use crate::core::coordinates::{eci_to_ecef_km, gmst_rad_with_dut1};
use crate::core::space::ecef_to_bevy_km;
pub use moon::{MoonEcefKm, moon_position_ecef_km};
pub use propagation::minutes_since_epoch;
pub use time::{Dut1, SimulationTime, advance_simulation_clock, sun_direction_from_utc};

/// Sun direction in Bevy world coordinates
#[derive(Resource, Deref, DerefMut)]
pub struct SunDirection(pub Vec3);

impl Default for SunDirection {
    fn default() -> Self {
        Self(Vec3::Z)
    }
}

fn update_sun_direction(
    sim_time: Res<SimulationTime>,
    dut1: Res<Dut1>,
    mut sun_direction: ResMut<SunDirection>,
) {
    let ecef = sun_direction_from_utc(sim_time.current_utc, **dut1);
    sun_direction.0 = ecef_to_bevy_km(ecef).normalize_or_zero();
}

/// Plugin for orbital mechanics and time management
pub struct OrbitalPlugin;

impl Plugin for OrbitalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationTime>()
            .init_resource::<Dut1>()
            .init_resource::<SunDirection>()
            .init_resource::<MoonEcefKm>()
            .add_systems(Update, advance_simulation_clock)
            .add_systems(
                Update,
                (update_sun_direction, moon::update_moon_state).after(advance_simulation_clock),
            );
    }
}
