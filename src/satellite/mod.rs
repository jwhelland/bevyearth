//! Satellite management module
//!
//! This module handles satellite components, resources, and systems for tracking
//! and managing satellite entities in the Bevy ECS.

use bevy::prelude::*;

pub mod components;
pub mod resources;
pub mod systems;

pub use components::{Satellite, SatelliteColor};
pub use resources::{
    ColorHueCounter, GroupMaterialCache, NoradIndex, OrbitTrailConfig, SatelliteRenderConfig,
    SelectedSatellite,
};
pub use systems::{
    draw_orbit_trails_system, init_satellite_render_assets, materialize_satellite_entities_system,
    move_camera_to_satellite, propagate_satellites_system, satellite_click_system,
    track_satellite_continuously, update_group_colors_system, update_orbit_trails_system,
    update_satellite_rendering_system,
};

/// Plugin for satellite management and propagation
pub struct SatellitePlugin;

impl Plugin for SatellitePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedSatellite>()
            .init_resource::<GroupMaterialCache>()
            .init_resource::<NoradIndex>()
            .init_resource::<ColorHueCounter>()
            .insert_resource(crate::ui::groups::initialize_group_registry())
            // OrbitTrailConfig and SatelliteRenderConfig are now in UiConfigBundle
            .add_systems(
                Startup,
                (init_satellite_render_assets, spawn_default_satellites).chain(),
            )
            .add_systems(
                Update,
                (
                    materialize_satellite_entities_system,
                    propagate_satellites_system.after(materialize_satellite_entities_system),
                    update_orbit_trails_system.after(propagate_satellites_system),
                    draw_orbit_trails_system.after(update_orbit_trails_system),
                    update_satellite_rendering_system,
                    update_group_colors_system,
                    move_camera_to_satellite,
                    track_satellite_continuously.after(propagate_satellites_system),
                    satellite_click_system,
                ),
            );
    }
}

fn spawn_default_satellites(
    mut commands: Commands,
    mut norad_index: ResMut<NoradIndex>,
    mut hue_counter: ResMut<ColorHueCounter>,
    fetch_channels: Option<Res<crate::tle::FetchChannels>>,
) {
    const ISS_NORAD: u32 = 25_544;

    if norad_index.map.contains_key(&ISS_NORAD) {
        return;
    }

    let hue = hue_counter.next_hue;
    hue_counter.next_hue = (hue + 0.618034).fract();
    let color = Color::hsl(hue * 360.0, 0.75, 0.65);

    let entity = commands
        .spawn((
            crate::satellite::components::Satellite,
            crate::satellite::components::NoradId(ISS_NORAD),
            crate::satellite::components::SatelliteColor(color),
            crate::satellite::components::SatelliteFlags::default(),
        ))
        .id();

    norad_index.map.insert(ISS_NORAD, entity);

    if let Some(fetch) = fetch_channels
        && let Err(err) = fetch
            .cmd_tx
            .send(crate::tle::FetchCommand::Fetch(ISS_NORAD))
    {
        eprintln!("[ISS] Failed to request TLE: {}", err);
    }
}
