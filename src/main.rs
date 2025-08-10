// Inspired by https://blog.graysonhead.net/posts/bevy-proc-earth-1/

use bevy::picking::prelude::*;
use bevy::render::mesh::Mesh;
use bevy::render::view::RenderLayers;
use bevy::{prelude::*, render::camera::Viewport, window::PrimaryWindow};

use bevy_egui::{
    egui, EguiContext, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass,
    PrimaryEguiContext,
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
// Additional imports
use bevy_egui::egui::Color32;
use bevy::math::DVec3;
use std::sync::{Arc, Mutex};

mod cities;
mod coord;
mod coverage;
mod earth;
mod footprint_gizmo;
mod orbital;
mod satellite;
mod tle;
mod ui;
mod visualization;
use crate::earth::EARTH_RADIUS_KM;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use cities::{spawn_city_population_spheres, CitiesEcef};
use coord::{hemisphere_prefilter, los_visible_ecef};
use coverage::{CoverageParameters, FootprintConfig};
use earth::generate_faces;
// use footprint_mesh::{FootprintMarker, FootprintMeshUtils};
use footprint_gizmo::{FootprintGizmo, FootprintGizmoConfig, draw_footprint_gizmos_system};
use satellite::{Satellite, SatelliteColor, SatelliteStore, SatEntry, SatEcef};
use satellite::{propagate_satellites_system, update_satellite_ecef};
use orbital::{SimulationTime, advance_simulation_clock};
use tle::{TleData, FetchChannels, FetchCommand, FetchResultMsg, start_tle_worker, parse_tle_epoch_to_utc, process_fetch_results_system};
use ui::{UIState, RightPanelUI, ui_example_system};
use visualization::{ArrowConfig, draw_city_to_satellite_arrows, draw_arrow_segment, draw_axes, ShowAxes};

// Helper function to create footprint material
// fn create_footprint_material() -> StandardMaterial {
//     StandardMaterial {
//         base_color: Color::srgba(0.0, 1.0, 0.0, 0.8), // More opaque green
//         alpha_mode: AlphaMode::Blend,
//         unlit: true, // Don't apply lighting to footprints
//         cull_mode: None, // Render both sides
//         double_sided: true, // Ensure visibility from both sides
//         ..default()
//     }
// }

// Satellite-related code moved to satellite module

// RightPanelUI moved to ui module

// TLE-related code moved to tle module

// TLE data structure moved to satellite module

// parse_tle_epoch_to_utc moved to tle module

// Orbital mechanics functions moved to satellite module (temporarily)

// ArrowConfig moved to visualization module

// SatEcef moved to satellite module

// UIState moved to ui module

// SimulationTime moved to satellite module (temporarily)

// start_tle_worker moved to tle module

// ShowAxes moved to visualization module

// Systems
// update_satellite_ecef moved to satellite module

// draw_arrow_segment, draw_city_to_satellite_arrows, and draw_axes moved to visualization module

// advance_simulation_clock moved to orbital module

// propagate_satellites_system moved to satellite module

// System to manage footprint entities (spawn/despawn based on settings)
// fn manage_footprint_entities_system(
//     mut commands: Commands,
//     mut store: ResMut<SatelliteStore>,
//     mut meshes: ResMut<Assets<Mesh>>,
//     mut materials: ResMut<Assets<StandardMaterial>>,
//     footprint_config: Res<FootprintConfig>,
//     satellite_query: Query<&Transform, With<Satellite>>,
// ) {
//     if !footprint_config.enabled {
//         // If footprints are globally disabled, despawn all footprint entities
//         for entry in store.items.iter_mut() {
//             if let Some(footprint_entity) = entry.footprint_entity.take() {
//                 commands.entity(footprint_entity).despawn();
//             }
//         }
//         return;
//     }

//     for entry in store.items.iter_mut() {
//         let should_show = entry.show_footprint && entry.propagator.is_some();
//         let has_entity = entry.footprint_entity.is_some();

//         if should_show && !has_entity {
//             // Need to spawn footprint entity
//             if let Some(sat_entity) = entry.entity {
//                 if let Ok(sat_transform) = satellite_query.get(sat_entity) {
//                     // Use current UI parameters instead of caching them
//                     let current_params = CoverageParameters {
//                         frequency_mhz: footprint_config.default_frequency_mhz,
//                         transmit_power_dbm: footprint_config.default_tx_power_dbm,
//                         antenna_gain_dbi: footprint_config.default_antenna_gain_dbi,
//                         min_signal_strength_dbm: footprint_config.default_min_signal_dbm,
//                         min_elevation_deg: footprint_config.default_min_elevation_deg,
//                     };

//                     // Generate initial footprint mesh with current UI parameters
//                     let footprint_mesh = FootprintMeshUtils::generate_satellite_footprint(
//                         sat_transform.translation,
//                         &current_params,
//                         footprint_config.mesh_resolution,
//                     );

//                     // Spawn footprint entity
//                     let footprint_entity = commands
//                         .spawn((
//                             Mesh3d(meshes.add(footprint_mesh)),
//                             MeshMaterial3d(materials.add(create_footprint_material())),
//                             Transform::from_translation(Vec3::ZERO),
//                             FootprintMarker {
//                                 satellite_norad: entry.norad,
//                             },
//                         ))
//                         .id();

//                     entry.footprint_entity = Some(footprint_entity);
//                 }
//             }
//         } else if !should_show && has_entity {
//             // Need to despawn footprint entity
//             if let Some(footprint_entity) = entry.footprint_entity.take() {
//                 commands.entity(footprint_entity).despawn();
//             }
//         }
//     }
// }

// System to update footprint meshes when satellites move (rate limited)
// fn update_footprint_meshes_system(
//     store: Res<SatelliteStore>,
//     footprint_config: Res<FootprintConfig>,
//     satellite_query: Query<&Transform, With<Satellite>>,
//     mut footprint_query: Query<&mut Mesh3d, With<FootprintMarker>>,
//     mut meshes: ResMut<Assets<Mesh>>,
//     time: Res<Time>,
//     mut last_update: Local<f32>,
// ) {
//     if !footprint_config.enabled {
//         return;
//     }

//     // Rate limit updates to avoid performance issues
//     let update_interval = 1.0 / footprint_config.update_frequency_hz;
//     *last_update += time.delta_secs();
//     if *last_update < update_interval {
//         return;
//     }
//     *last_update = 0.0;

//     for entry in store.items.iter() {
//         if let (Some(footprint_entity), Some(sat_entity)) =
//             (entry.footprint_entity, entry.entity) {
            
//             if entry.show_footprint && entry.propagator.is_some() {
//                 if let Ok(sat_transform) = satellite_query.get(sat_entity) {
//                     // Use current UI parameters instead of cached ones
//                     let current_params = CoverageParameters {
//                         frequency_mhz: footprint_config.default_frequency_mhz,
//                         transmit_power_dbm: footprint_config.default_tx_power_dbm,
//                         antenna_gain_dbi: footprint_config.default_antenna_gain_dbi,
//                         min_signal_strength_dbm: footprint_config.default_min_signal_dbm,
//                         min_elevation_deg: footprint_config.default_min_elevation_deg,
//                     };
                    
//                     // Check if footprint has visible coverage
//                     if FootprintMeshUtils::has_visible_coverage(sat_transform.translation, &current_params) {
//                         // Generate updated mesh with current UI parameters
//                         let new_mesh = FootprintMeshUtils::generate_satellite_footprint(
//                             sat_transform.translation,
//                             &current_params,
//                             footprint_config.mesh_resolution,
//                         );

//                         // Find and update the footprint mesh
//                         if let Ok(mut mesh_handle) = footprint_query.get_mut(footprint_entity) {
//                             *mesh_handle = Mesh3d(meshes.add(new_mesh));
//                         }
//                     }
//                 }
//             }
//         }
//     }
// }

// System to manage footprint gizmo components (add/remove based on settings)
fn manage_footprint_gizmo_components_system(
    mut commands: Commands,
    mut store: ResMut<SatelliteStore>,
    footprint_config: Res<FootprintConfig>,
    gizmo_config: Res<FootprintGizmoConfig>,
    satellite_query: Query<Entity, With<Satellite>>,
    gizmo_query: Query<Entity, With<FootprintGizmo>>,
) {
    if !footprint_config.enabled || !gizmo_config.enabled {
        // If footprints are globally disabled, remove all FootprintGizmo components
        for gizmo_entity in gizmo_query.iter() {
            commands.entity(gizmo_entity).remove::<FootprintGizmo>();
        }
        return;
    }

    for entry in store.items.iter_mut() {
        let should_show = entry.show_footprint && entry.propagator.is_some();
        
        if let Some(sat_entity) = entry.entity {
            if let Ok(entity) = satellite_query.get(sat_entity) {
                let has_gizmo_component = gizmo_query.get(entity).is_ok();
                
                if should_show && !has_gizmo_component {
                    // Add FootprintGizmo component (parameters will be read dynamically from UI)
                    let dummy_params = CoverageParameters::default();
                    commands.entity(entity).insert(FootprintGizmo::new(entry.norad, dummy_params));
                } else if !should_show && has_gizmo_component {
                    // Remove FootprintGizmo component
                    commands.entity(entity).remove::<FootprintGizmo>();
                }
            }
        }
    }
}

// Setup scene, cameras, and TLE worker
pub fn setup(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    egui_global_settings.auto_create_primary_context = false;

    // Start TLE worker
    let channels = start_tle_worker();
    println!("[INIT] TLE worker started");
    commands.insert_resource(channels);

    // Axes marker
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap())),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0., 0.))),
        ShowAxes,
    ));
    commands.spawn((PanOrbitCamera::default(), Transform::from_xyz(25000.0, 8.0, 4.0)));
    commands.spawn((
        Camera2d,
        PrimaryEguiContext,
        RenderLayers::none(),
        Camera { order: 1, ..default() },
        Transform::from_xyz(25000.0, 8.0, 4.0),
    ));
}

// ui_example_system moved to ui module

// process_fetch_results_system moved to tle module

fn main() {
    App::new()
        .init_resource::<UIState>()
        .init_resource::<ArrowConfig>()
        .init_resource::<SatEcef>()
        .init_resource::<SimulationTime>()
        .init_resource::<SatelliteStore>()
        .init_resource::<RightPanelUI>()
        .init_resource::<FootprintConfig>()
        .init_resource::<FootprintGizmoConfig>()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(MeshPickingPlugin)
        .add_systems(Startup, (setup, generate_faces, spawn_city_population_spheres).chain())
        .add_systems(
            Update,
            (
                draw_axes.after(setup),
                advance_simulation_clock,               // advance UTC
                process_fetch_results_system,           // receive TLEs/models
                propagate_satellites_system.after(advance_simulation_clock), // update sat transforms
                update_satellite_ecef.after(propagate_satellites_system),
                draw_city_to_satellite_arrows.after(propagate_satellites_system),
                manage_footprint_gizmo_components_system.after(propagate_satellites_system), // manage footprint gizmo components
                draw_footprint_gizmos_system.after(manage_footprint_gizmo_components_system), // draw footprint gizmos
                // Disable old mesh systems for now
                // manage_footprint_entities_system.after(propagate_satellites_system), // manage footprint entities
                // update_footprint_meshes_system.after(manage_footprint_entities_system), // update footprint meshes
            ),
        )
        .add_systems(EguiPrimaryContextPass, ui_example_system)
        .run();
}
