//! Space weather module (NOAA SWPC feeds + rendering).

use bevy::prelude::*;

pub mod fetcher;
pub mod systems;
pub mod types;

pub use types::{AuroraGrid, KpIndex, SolarWind, SpaceWeatherConfig, SpaceWeatherState};

pub struct SpaceWeatherPlugin;

impl Plugin for SpaceWeatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpaceWeatherConfig>()
            .init_resource::<SpaceWeatherState>()
            .init_resource::<AuroraGrid>()
            .init_resource::<KpIndex>()
            .init_resource::<SolarWind>()
            .init_resource::<systems::AuroraRenderState>()
            .add_systems(Startup, systems::setup_space_weather_worker)
            .add_systems(
                Update,
                (
                    systems::poll_space_weather,
                    systems::apply_space_weather_results,
                    systems::initialize_aurora_overlay,
                    systems::update_aurora_texture,
                    systems::sync_aurora_visibility,
                )
                    .chain(),
            );
    }
}
