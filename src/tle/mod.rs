//! TLE (Two-Line Element) data management module
//!
//! This module handles TLE fetching, parsing, and data structures for satellite
//! orbital elements from external sources like Celestrak.

use bevy::prelude::*;

pub mod cache;
pub mod fetcher;
pub mod parser;
pub mod systems;
pub mod types;

pub use fetcher::start_tle_worker;
pub use systems::process_fetch_results_system;
pub use types::{FetchChannels, FetchCommand, TleCacheConfig, TleData};

/// Plugin for TLE data management and processing
pub struct TlePlugin;

impl Plugin for TlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TleCacheConfig>()
            .add_systems(Startup, setup_tle_worker)
            .add_systems(Update, process_fetch_results_system);
    }
}

/// Setup system to start the TLE worker
fn setup_tle_worker(mut commands: Commands, config: Res<TleCacheConfig>) {
    let channels = start_tle_worker(config.clone());
    println!("[INIT] TLE worker started");
    commands.insert_resource(channels);
}
