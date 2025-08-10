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

// Import plugins
use cities::CitiesPlugin;
use coverage::CoveragePlugin;
use earth::EarthPlugin;
use footprint_gizmo::FootprintGizmoPlugin;
use orbital::OrbitalPlugin;
use satellite::SatellitePlugin;
use tle::TlePlugin;
use ui::UiPlugin;
use visualization::{VisualizationPlugin, ShowAxes};

// Setup scene and cameras
pub fn setup(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    egui_global_settings.auto_create_primary_context = false;

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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
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
        .add_plugins(VisualizationPlugin)
        .add_plugins(CoveragePlugin)
        .add_plugins(FootprintGizmoPlugin)
        .add_systems(Startup, setup)
        .run();
}
