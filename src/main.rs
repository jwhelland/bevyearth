// Inspired by https://blog.graysonhead.net/posts/bevy-proc-earth-1/

use bevy::picking::prelude::*;
use bevy::prelude::*;
use bevy::render::mesh::Mesh;

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

// mod cities;
// use crate::cities::spawn_city_population_spheres;
mod cities;
mod coord;
mod earth;
// use earth::EARTH_RADIUS_KM;
use cities::spawn_city_population_spheres;
use earth::generate_faces;

/// The `ShowAxes` component is attached to an entity to get the `draw_axes` system to
/// display axes according to its Transform component.
#[derive(Component)]
struct ShowAxes;

fn draw_axes(mut gizmos: Gizmos, query: Query<&Transform, With<ShowAxes>>) {
    for &transform in &query {
        gizmos.axes(transform, 8000.0);
    }
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // asset_server: Res<AssetServer>,
) {

    // Small sphere at origin to show axes
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap())),
        MeshMaterial3d(materials.add(Color::srgb(0., 0., 0.))),
        ShowAxes
    ));

    commands.spawn((
        PanOrbitCamera::default(),
        Transform::from_xyz(25000.0, 8.0, 4.0),   
    ));
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(MeshPickingPlugin)
        .add_systems(
            Startup,
            (setup, spawn_city_population_spheres, generate_faces),
        )
        .add_systems(Update, draw_axes.after(setup))
        .run();
}
