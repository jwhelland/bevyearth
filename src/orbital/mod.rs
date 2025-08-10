//! Orbital mechanics module
//! 
//! This module handles orbital calculations, coordinate transformations,
//! and time management for satellite propagation.

pub mod coordinates;
pub mod propagation;
pub mod time;

pub use coordinates::{eci_to_ecef_km, gmst_rad};
pub use propagation::minutes_since_epoch;
pub use time::{SimulationTime, advance_simulation_clock};