//! TLE (Two-Line Element) data management module
//!
//! This module handles TLE fetching, parsing, and data structures for satellite
//! orbital elements from external sources like Celestrak.

use bevy::prelude::*;

pub mod fetcher;
pub mod parser;
pub mod systems;
pub mod types;

pub use types::{TleData, FetchCommand, FetchChannels};
pub use fetcher::start_tle_worker;
pub use systems::process_fetch_results_system;

/// Plugin for TLE data management and processing
pub struct TlePlugin;

impl Plugin for TlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_tle_worker)
            .add_systems(Update, process_fetch_results_system);
    }
}

/// Setup system to start the TLE worker
fn setup_tle_worker(mut commands: Commands) {
    let channels = start_tle_worker();
    println!("[INIT] TLE worker started");
    commands.insert_resource(channels);
}