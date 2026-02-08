// Inspired by https://blog.graysonhead.net/posts/bevy-proc-earth-1/

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::GlobalAmbientLight;
use bevy::light::SunDisk;
use bevy::mesh::Mesh;
use bevy::picking::prelude::*;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::window::{PresentMode, Window, WindowPlugin};

use bevy_feathers::FeathersPlugins;
use bevy_feathers::dark_theme::create_dark_theme;
use bevy_feathers::palette;
use bevy_feathers::theme::UiTheme;
use bevy_input_focus::directional_navigation::DirectionalNavigationPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

#[cfg(feature = "dev_camera")]
use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
#[cfg(feature = "dev")]
use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;

mod core;
mod orbital;
mod satellite;
mod space_weather;
mod tle;
mod ui;
mod visualization;

// Import plugins
use orbital::OrbitalPlugin;
use satellite::SatellitePlugin;
use space_weather::SpaceWeatherPlugin;
use tle::TlePlugin;
use ui::{MainCamera, SkyboxPlugin, UiPlugin, skybox::Cubemap};
use visualization::{
    CitiesPlugin, EarthPlugin, GroundTrackGizmoPlugin, GroundTrackPlugin, HeatmapPlugin, ShowAxes,
    SunLight, VisualizationPlugin,
};

#[cfg(all(feature = "debug_basic_scene", feature = "debug_scene_camera"))]
compile_error!("Enable only one of: debug_basic_scene or debug_scene_camera.");

#[cfg(feature = "dev_camera")]
#[derive(Component)]
struct DevCamera;

// Setup scene and cameras
#[cfg(not(feature = "debug_basic_scene"))]
pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Ensure the scene is visible even when the sun is behind the camera / Earth is in shadow.
    commands.insert_resource(GlobalAmbientLight {
        brightness: 150.0,
        ..default()
    });

    let skybox_handle: Handle<Image> = asset_server.load("skybox.png");
    // Axes marker
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap())),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0., 0.))),
        ShowAxes,
    ));
    // Configure PanOrbitCamera for our scene scale (Earth radius = 6371 km)
    let initial_distance = 25000.0; // ~4x Earth's radius

    let pan_orbit = PanOrbitCamera {
        focus: Vec3::ZERO,              // Look at Earth's center
        radius: Some(initial_distance), // Initial distance from focus point
        yaw: Some(0.0),                 // Initial yaw angle
        pitch: Some(0.0),               // Initial pitch angle
        force_update: true,             // Force immediate positioning
        ..default()
    };

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            // World units are kilometers; default far plane is too small and clips the Earth/satellites.
            near: 1.0,
            far: 250_000.0,
            ..default()
        }),
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        pan_orbit,
        MainCamera,
        /* Skybox moved to custom entity in ui/skybox.rs
        Skybox {
            image: skybox_handle.clone(),
            brightness: 500.0,  // Adjusted for visibility
            ..default()
        },
        */
        Tonemapping::TonyMcMapface,
        // Note: Bloom is intentionally disabled - it causes rendering issues with PanOrbitCamera
        Transform::from_xyz(0.0, 0.0, initial_distance).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // Position sun light far from Earth in initial direction
    let sun_distance = 150_000.0; // 150,000 km from Earth
    let initial_sun_direction = Vec3::new(0.0, 0.0, 1.0).normalize();

    commands.spawn((
        DirectionalLight {
            illuminance: 8_000.0,
            ..default()
        },
        SunDisk::EARTH,
        SunLight,
        Transform::from_xyz(
            initial_sun_direction.x * sun_distance,
            initial_sun_direction.y * sun_distance,
            initial_sun_direction.z * sun_distance,
        )
        .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.insert_resource(Cubemap {
        is_loaded: false,
        image_handle: skybox_handle,
        activated: true,
    });
}

#[cfg(feature = "debug_basic_scene")]
fn setup_basic_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Minimal scene to verify that 3D rendering works at all.
    let sphere = meshes.add(Sphere::new(6_371.0).mesh().ico(4).unwrap());
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.6, 0.9),
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(sphere),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new("Debug Sphere"),
    ));

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            near: 1.0,
            far: 200_000.0,
            ..default()
        }),
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 25_000.0).looking_at(Vec3::ZERO, Vec3::Y),
        Name::new("Debug Camera"),
    ));
}

#[cfg(feature = "debug_scene_camera")]
#[derive(Resource)]
struct DebugSceneCamera {
    entity: Entity,
}

#[cfg(feature = "debug_scene_camera")]
fn setup_debug_scene_camera(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let sphere = meshes.add(Sphere::new(6_371.0).mesh().ico(4).unwrap());
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.6, 0.9),
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(sphere),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new("Debug Sphere"),
    ));

    let skybox_handle: Handle<Image> = asset_server.load("skybox.png");

    let camera = commands
        .spawn((
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                near: 1.0,
                far: 250_000.0,
                ..default()
            }),
            Camera {
                order: 0,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            Hdr,
            Bloom::NATURAL,
            Tonemapping::TonyMcMapface,
            PanOrbitCamera::default(),
            Transform::from_xyz(25_000.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
            Name::new("Debug Camera"),
        ))
        .id();

    commands.insert_resource(DebugSceneCamera { entity: camera });
    commands.insert_resource(Cubemap {
        is_loaded: false,
        image_handle: skybox_handle,
        activated: true,
    });
}

#[cfg(feature = "debug_scene_camera")]
fn toggle_debug_scene_components(
    input: Res<ButtonInput<KeyCode>>,
    cam: Res<DebugSceneCamera>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_camera: Query<Option<&Skybox>, With<Camera3d>>,
) {
    let Ok(skybox) = q_camera.get(cam.entity) else {
        return;
    };

    if input.just_pressed(KeyCode::Digit1) {
        if skybox.is_some() {
            commands.entity(cam.entity).remove::<Skybox>();
            info!("Debug scene: Skybox OFF (press 1)");
        } else {
            commands.entity(cam.entity).insert(Skybox {
                image: asset_server.load("skybox.png"),
                brightness: 1000.0,
                ..default()
            });
            info!("Debug scene: Skybox ON (press 1)");
        }
    }
}

#[cfg(feature = "dev_camera")]
fn setup_dev_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            near: 1.0,
            far: 250_000.0,
            ..default()
        }),
        Camera {
            order: 2,
            is_active: false,
            ..default()
        },
        FreeCamera::default(),
        DevCamera,
    ));
}

#[cfg(feature = "dev_camera")]
fn toggle_dev_camera(
    input: Res<ButtonInput<KeyCode>>,
    mut main_camera: Query<&mut Camera, (With<MainCamera>, Without<DevCamera>)>,
    mut dev_camera: Query<&mut Camera, With<DevCamera>>,
) {
    if !input.just_pressed(KeyCode::F2) {
        return;
    }

    if let Ok(mut main) = main_camera.get_single_mut()
        && let Ok(mut dev) = dev_camera.get_single_mut()
    {
        let dev_active = dev.is_active;
        dev.is_active = !dev_active;
        main.is_active = dev_active;
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins(
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
    );

    #[cfg(feature = "dev")]
    app.add_plugins(FpsOverlayPlugin::default());

    #[cfg(all(
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ))]
    {
        // Feathers initializes `UiTheme` but does not populate it by default.
        // If we don't set this, many widgets will render with the "missing token" fallback color.
        let mut theme = UiTheme(create_dark_theme());
        theme.set_color("feathers.text.main", palette::LIGHT_GRAY_1);
        theme.set_color("feathers.text.dim", palette::LIGHT_GRAY_2);
        theme.set_color("feathers.focus", palette::ACCENT);
        theme.set_color("feathers.slider.bg", Color::srgba(0.04, 0.08, 0.12, 0.9));
        theme.set_color("feathers.slider.bar", Color::srgba(0.18, 0.7, 0.8, 0.7));
        theme.set_color(
            "feathers.slider.bar.disabled",
            Color::srgba(0.1, 0.3, 0.35, 0.45),
        );
        theme.set_color("feathers.slider.text", Color::srgba(0.5, 0.9, 0.95, 0.95));
        theme.set_color(
            "feathers.slider.text.disabled",
            Color::srgba(0.4, 0.55, 0.6, 0.7),
        );
        theme.set_color("feathers.button.bg", Color::srgba(0.06, 0.12, 0.16, 0.9));
        theme.set_color(
            "feathers.button.bg.hover",
            Color::srgba(0.08, 0.2, 0.26, 0.95),
        );
        theme.set_color(
            "feathers.button.bg.pressed",
            Color::srgba(0.1, 0.26, 0.32, 0.95),
        );
        theme.set_color(
            "feathers.button.bg.disabled",
            Color::srgba(0.08, 0.12, 0.15, 0.5),
        );
        theme.set_color("feathers.button.txt", Color::srgba(0.6, 1.0, 1.0, 1.0));
        theme.set_color(
            "feathers.button.txt.disabled",
            Color::srgba(0.45, 0.6, 0.65, 0.7),
        );
        theme.set_color(
            "feathers.button.primary.bg",
            Color::srgba(0.08, 0.22, 0.28, 0.95),
        );
        theme.set_color(
            "feathers.button.primary.bg.hover",
            Color::srgba(0.1, 0.28, 0.36, 0.98),
        );
        theme.set_color(
            "feathers.button.primary.bg.pressed",
            Color::srgba(0.12, 0.32, 0.4, 0.98),
        );
        theme.set_color(
            "feathers.button.primary.bg.disabled",
            Color::srgba(0.08, 0.16, 0.2, 0.6),
        );
        theme.set_color(
            "feathers.button.primary.txt",
            Color::srgba(0.7, 1.0, 1.0, 1.0),
        );
        theme.set_color(
            "feathers.button.primary.txt.disabled",
            Color::srgba(0.45, 0.6, 0.65, 0.7),
        );
        app.insert_resource(theme);

        app.add_plugins(FeathersPlugins);
        app.add_plugins(DirectionalNavigationPlugin);

        #[cfg(feature = "dev_camera")]
        app.add_plugins(FreeCameraPlugin);

        app.add_plugins(PanOrbitCameraPlugin);
        app.add_plugins(MeshPickingPlugin);

        // Add our custom plugins
        app.add_plugins(EarthPlugin);
        app.add_plugins(CitiesPlugin);
        app.add_plugins(OrbitalPlugin);
        app.add_plugins(SatellitePlugin);
        app.add_plugins(TlePlugin);
        app.add_plugins(SpaceWeatherPlugin);
        app.add_plugins(UiPlugin);
        app.add_plugins(SkyboxPlugin);
        app.add_plugins(VisualizationPlugin);
        app.add_plugins(GroundTrackPlugin);
        app.add_plugins(GroundTrackGizmoPlugin);
        app.add_plugins(HeatmapPlugin);
        app.add_systems(Startup, setup);
    }

    #[cfg(feature = "debug_basic_scene")]
    {
        app.add_systems(Startup, setup_basic_scene);
    }

    #[cfg(feature = "debug_scene_camera")]
    {
        app.add_plugins(PanOrbitCameraPlugin);
        app.add_plugins(SkyboxPlugin);
        app.add_systems(Startup, setup_debug_scene_camera);
        app.add_systems(Update, toggle_debug_scene_components);
    }

    #[cfg(all(
        feature = "dev_camera",
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ))]
    {
        app.add_systems(Startup, setup_dev_camera);
        app.add_systems(Update, toggle_dev_camera);
    }

    app.run();
}
