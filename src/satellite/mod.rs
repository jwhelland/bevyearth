//! Satellite management module
//! 
//! This module handles satellite components, resources, and systems for tracking
//! and managing satellite entities in the Bevy ECS.

pub mod components;
pub mod resources;
pub mod systems;

pub use components::{Satellite, SatelliteColor};
pub use resources::{SatelliteStore, SatEntry, SatEcef};
pub use systems::{propagate_satellites_system, update_satellite_ecef};