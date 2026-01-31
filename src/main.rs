// Inspired by https://blog.graysonhead.net/posts/bevy-proc-earth-1/

use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::Skybox;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::mesh::Mesh;
use bevy::picking::prelude::*;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::render::view::Hdr;
use bevy::transform::TransformPlugin;
use bevy::window::{PresentMode, Window, WindowPlugin};

use bevy_egui::{EguiGlobalSettings, EguiPlugin, PrimaryEguiContext};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use big_space::prelude::{BigSpaceDefaultPlugins, CellCoord, FloatingOrigin};

mod core;
mod orbital;
mod satellite;
mod tle;
mod ui;
mod visualization;

use crate::core::big_space::{BigSpaceRoot, StartupSet, setup_big_space_root};
use crate::core::orbit_camera::BigSpacePanOrbitPlugin;
// Import plugins
use orbital::OrbitalPlugin;
use satellite::SatellitePlugin;
use tle::TlePlugin;
use ui::{SkyboxPlugin, UiPlugin, skybox::Cubemap};
use visualization::{
    CitiesPlugin, EarthPlugin, GroundTrackGizmoPlugin, GroundTrackPlugin, HeatmapPlugin, ShowAxes,
    VisualizationPlugin,
};

// Setup scene and cameras
pub fn setup(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    big_space_root: Res<BigSpaceRoot>,
) {
    egui_global_settings.auto_create_primary_context = false;
    let skybox_handle: Handle<Image> = asset_server.load("skybox.png");

    // Axes marker
    commands.entity(big_space_root.0).with_children(|parent| {
        parent.spawn((
            Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap())),
            MeshMaterial3d(materials.add(Color::srgb(1.0, 0., 0.))),
            CellCoord::ZERO,
            Transform::from_xyz(0.0, 0.0, 0.0),
            ShowAxes,
        ));
        parent
            .spawn((
                FloatingOrigin,
                crate::core::orbit_camera::PanOrbitFloatingOrigin,
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                CellCoord::ZERO,
                Transform::default(),
            ))
            .with_children(|origin| {
                origin.spawn((
                    Camera3d::default(),
                    Camera {
                        order: 1,
                        clear_color: ClearColorConfig::Custom(Color::BLACK),
                        ..default()
                    },
                    Hdr,
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
            });
    });
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
                .build()
                .disable::<TransformPlugin>()
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
        .add_plugins(BigSpaceDefaultPlugins)
        .add_plugins(BigSpacePanOrbitPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(MeshPickingPlugin)
        .configure_sets(Startup, StartupSet::BigSpace.before(StartupSet::Scene))
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
        .add_plugins(HeatmapPlugin)
        .add_systems(Startup, setup_big_space_root.in_set(StartupSet::BigSpace))
        .add_systems(Startup, setup.in_set(StartupSet::Scene))
        .run();
}
