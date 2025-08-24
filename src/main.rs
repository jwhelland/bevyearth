// Inspired by https://blog.graysonhead.net/posts/bevy-proc-earth-1/

use bevy::core_pipeline::Skybox;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::picking::prelude::*;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::mesh::Mesh;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::render::view::RenderLayers;
use bevy::window::{PresentMode, Window, WindowPlugin};

use bevy_egui::{EguiGlobalSettings, EguiPlugin, PrimaryEguiContext};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

mod cities;
mod core;
mod earth;
mod ground_track;
mod ground_track_gizmo;
mod orbital;
mod satellite;
mod tle;
mod ui;
mod visualization;

// Import plugins
use cities::CitiesPlugin;
use earth::EarthPlugin;
use ground_track::GroundTrackPlugin;
use ground_track_gizmo::GroundTrackGizmoPlugin;
use orbital::OrbitalPlugin;
use satellite::SatellitePlugin;
use tle::TlePlugin;
use ui::{SkyboxPlugin, UiPlugin};
use visualization::{ShowAxes, VisualizationPlugin};

use crate::ui::skybox::Cubemap;

// Setup scene and cameras
pub fn setup(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    egui_global_settings.auto_create_primary_context = false;
    let skybox_handle: Handle<Image> = asset_server.load("skybox.png");

    // Axes marker
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap())),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0., 0.))),
        ShowAxes,
    ));
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 1,
            hdr: true, // 1. HDR is required for bloom
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        PanOrbitCamera::default(),
        Skybox {
            image: skybox_handle.clone(),
            brightness: 1000.0,
            ..default()
        },
        Bloom::NATURAL,
        Tonemapping::TonyMcMapface,
        Transform::from_xyz(25000.0, 8.0, 4.0),
    ));
    commands.spawn((
        Camera2d,
        PrimaryEguiContext,
        RenderLayers::none(),
        Camera {
            order: 0,
            ..default()
        },
        Transform::from_xyz(25000.0, 8.0, 4.0),
    ));

    commands.insert_resource(Cubemap {
        is_loaded: false,
        image_handle: skybox_handle,
        activated: true,
    });
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy Earth Satellite Tracker".to_string(),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings { ..default() }),
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(MeshPickingPlugin)
        // Add our custom plugins
        .add_plugins(EarthPlugin)
        .add_plugins(CitiesPlugin)
        .add_plugins(OrbitalPlugin)
        .add_plugins(SatellitePlugin)
        .add_plugins(TlePlugin)
        .add_plugins(UiPlugin)
        .add_plugins(SkyboxPlugin)
        .add_plugins(VisualizationPlugin)
        .add_plugins(GroundTrackPlugin)
        .add_plugins(GroundTrackGizmoPlugin)
        .add_systems(Startup, setup)
        .run();
}
