use bevy::mesh::Mesh;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::render::view::Tonemapping;
use bevy::window::{PresentMode, Window, WindowPlugin};
use camera::{CameraPlugin, orbit_controller};

#[cfg(all(feature = "debug_scene_camera", not(feature = "debug_basic_scene")))]
use bevy::camera::Hdr;
#[cfg(feature = "dev")]
use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
#[cfg(any(
    all(
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ),
    all(feature = "debug_basic_scene", feature = "debug_scene_camera")
))]
use bevy::feathers::FeathersPlugins;
#[cfg(any(
    all(
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ),
    all(feature = "debug_basic_scene", feature = "debug_scene_camera")
))]
use bevy::input_focus::directional_navigation::DirectionalNavigationPlugin;
#[cfg(all(feature = "debug_scene_camera", not(feature = "debug_basic_scene")))]
use bevy::light::Skybox;
#[cfg(any(
    all(
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ),
    all(feature = "debug_basic_scene", feature = "debug_scene_camera")
))]
use bevy::light::{GlobalAmbientLight, SunDisk};
#[cfg(any(
    all(
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ),
    all(feature = "debug_basic_scene", feature = "debug_scene_camera")
))]
use bevy::picking::prelude::MeshPickingPlugin;
#[cfg(all(feature = "debug_scene_camera", not(feature = "debug_basic_scene")))]
use bevy::post_process::bloom::Bloom;
#[cfg(all(
    feature = "dev_camera",
    any(
        all(
            not(feature = "debug_basic_scene"),
            not(feature = "debug_scene_camera")
        ),
        all(feature = "debug_basic_scene", feature = "debug_scene_camera")
    )
))]
use bevy_camera_controller::free_camera::{FreeCamera, FreeCameraPlugin, FreeCameraState};

mod camera;
mod core;
mod launch_library;
mod orbital;
mod satellite;
mod space_weather;
mod tle;
mod ui;
mod visualization;

// Import plugins
#[cfg(any(
    all(
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ),
    all(feature = "debug_basic_scene", feature = "debug_scene_camera")
))]
use launch_library::LaunchLibraryPlugin;
#[cfg(any(
    all(
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ),
    all(feature = "debug_basic_scene", feature = "debug_scene_camera")
))]
use orbital::OrbitalPlugin;
#[cfg(all(feature = "debug_scene_camera", not(feature = "debug_basic_scene")))]
use orbital::{Dut1, SimulationTime};
#[cfg(any(
    all(
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ),
    all(feature = "debug_basic_scene", feature = "debug_scene_camera")
))]
use satellite::SatellitePlugin;
#[cfg(any(
    all(
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ),
    all(feature = "debug_basic_scene", feature = "debug_scene_camera")
))]
use space_weather::SpaceWeatherPlugin;
#[cfg(any(
    all(
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ),
    all(feature = "debug_basic_scene", feature = "debug_scene_camera")
))]
use tle::TlePlugin;
#[cfg(any(
    all(
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ),
    all(feature = "debug_basic_scene", feature = "debug_scene_camera")
))]
use ui::{MainCamera, SkyboxPlugin, UiPlugin, skybox::Cubemap};
#[cfg(all(feature = "debug_scene_camera", not(feature = "debug_basic_scene")))]
use ui::{MainCamera, SkyboxPlugin, skybox::Cubemap};
#[cfg(any(
    all(
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ),
    all(feature = "debug_basic_scene", feature = "debug_scene_camera")
))]
use visualization::{
    CitiesPlugin, EarthPlugin, GroundTrackGizmoPlugin, GroundTrackPlugin, HeatmapPlugin,
    LaunchesPlugin, MoonPlugin, ShowAxes, SunLight, VisualizationPlugin,
};

#[cfg(all(
    feature = "dev_camera",
    any(
        all(
            not(feature = "debug_basic_scene"),
            not(feature = "debug_scene_camera")
        ),
        all(feature = "debug_basic_scene", feature = "debug_scene_camera")
    )
))]
#[derive(Component)]
struct DevCamera;

// Setup scene and cameras
#[cfg(any(
    all(
        not(feature = "debug_basic_scene"),
        not(feature = "debug_scene_camera")
    ),
    all(feature = "debug_basic_scene", feature = "debug_scene_camera")
))]
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

    let pan_orbit = orbit_controller(initial_distance);

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            // World units are kilometers; default far plane is too small and clips the Earth/satellites.
            near: 1.0,
            far: 1_000_000.0,
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

#[cfg(all(feature = "debug_basic_scene", not(feature = "debug_scene_camera")))]
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
            far: 1_000_000.0,
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

#[cfg(all(feature = "debug_scene_camera", not(feature = "debug_basic_scene")))]
#[derive(Resource)]
struct DebugSceneCamera {
    entity: Entity,
}

#[cfg(all(feature = "debug_scene_camera", not(feature = "debug_basic_scene")))]
#[derive(Component)]
struct DiagnosticSky;

#[cfg(all(feature = "debug_scene_camera", not(feature = "debug_basic_scene")))]
fn setup_debug_scene_camera(
    mut sky_materials: ResMut<Assets<visualization::sky_material::SkyMaterial>>,
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
                far: 1_000_000.0,
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
            orbit_controller(25_000.0),
            MainCamera,
            Transform::from_xyz(25_000.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
            Name::new("Debug Camera"),
        ))
        .id();

    // A separate cubemap sphere forces WESL compilation in this diagnostic.
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(900_000.0).mesh().uv(32, 16))),
        MeshMaterial3d(sky_materials.add(visualization::sky_material::SkyMaterial {
            texture: skybox_handle.clone(),
        })),
        Transform::default(),
        bevy::picking::Pickable::IGNORE,
        Name::new("Custom SkyMaterial diagnostic"),
        Visibility::Hidden,
        DiagnosticSky,
    ));
    commands.insert_resource(DebugSceneCamera { entity: camera });
    commands.insert_resource(SimulationTime::default());
    commands.insert_resource(Dut1::default());
    commands.insert_resource(Cubemap {
        is_loaded: false,
        image_handle: skybox_handle,
        activated: true,
    });
}

#[cfg(all(feature = "debug_scene_camera", not(feature = "debug_basic_scene")))]
fn toggle_debug_scene_components(
    input: Res<ButtonInput<KeyCode>>,
    cam: Res<DebugSceneCamera>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_camera: Query<Option<&Skybox>, With<Camera3d>>,
    mut custom_sky: Query<&mut Visibility, With<DiagnosticSky>>,
) {
    if input.just_pressed(KeyCode::Digit2) {
        for mut visibility in &mut custom_sky {
            *visibility = if *visibility == Visibility::Hidden {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        info!("Debug scene: toggled custom WESL sky (press 2)");
    }
    let Ok(skybox) = q_camera.get(cam.entity) else {
        return;
    };

    if input.just_pressed(KeyCode::Digit1) {
        if skybox.is_some() {
            commands.entity(cam.entity).remove::<Skybox>();
            info!("Debug scene: Skybox OFF (press 1)");
        } else {
            commands.entity(cam.entity).insert(Skybox {
                image: Some(asset_server.load("skybox.png")),
                brightness: 1000.0,
                ..default()
            });
            info!("Debug scene: Skybox ON (press 1)");
        }
    }
}

#[cfg(all(
    feature = "dev_camera",
    any(
        all(
            not(feature = "debug_basic_scene"),
            not(feature = "debug_scene_camera")
        ),
        all(feature = "debug_basic_scene", feature = "debug_scene_camera")
    )
))]
fn setup_dev_camera(mut commands: Commands) {
    let mut state = FreeCameraState::default();
    state.enabled = false;
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            near: 1.0,
            far: 1_000_000.0,
            ..default()
        }),
        Camera {
            order: 2,
            is_active: false,
            ..default()
        },
        FreeCamera::default(),
        state,
        DevCamera,
    ));
}

#[cfg(all(
    feature = "dev_camera",
    any(
        all(
            not(feature = "debug_basic_scene"),
            not(feature = "debug_scene_camera")
        ),
        all(feature = "debug_basic_scene", feature = "debug_scene_camera")
    )
))]
#[allow(clippy::type_complexity)]
fn toggle_dev_camera(
    input: Res<ButtonInput<KeyCode>>,
    mut main_camera: Query<(&mut Camera, &Transform), (With<MainCamera>, Without<DevCamera>)>,
    mut dev_camera: Query<(&mut Camera, &mut Transform, &mut FreeCameraState), With<DevCamera>>,
) {
    if !input.just_pressed(KeyCode::F2) {
        return;
    }

    if let Ok((mut main, main_transform)) = main_camera.single_mut()
        && let Ok((mut dev, mut transform, mut state)) = dev_camera.single_mut()
    {
        let dev_active = dev.is_active;
        if !dev_active {
            *transform = *main_transform;
            let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
            state.yaw = yaw;
            state.pitch = pitch;
        }
        state.velocity = Vec3::ZERO;
        state.rotation_curve = None;
        state.enabled = !dev_active;
        dev.is_active = !dev_active;
        main.is_active = dev_active;
        info!(
            "Development camera {} (F2)",
            if dev.is_active { "ON" } else { "OFF" }
        );
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Earth".to_string(),
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(WgpuSettings { ..default() })),
                ..default()
            }),
    );

    #[cfg(feature = "dev")]
    app.add_plugins(FpsOverlayPlugin::default());

    #[cfg(all(feature = "debug_basic_scene", feature = "debug_scene_camera"))]
    warn!("Both debug scene features enabled; using full app.");

    #[cfg(any(
        all(
            not(feature = "debug_basic_scene"),
            not(feature = "debug_scene_camera")
        ),
        all(feature = "debug_basic_scene", feature = "debug_scene_camera")
    ))]
    {
        app.insert_resource(ui::theme::app_theme());

        app.add_plugins(FeathersPlugins);
        app.add_plugins(DirectionalNavigationPlugin);

        #[cfg(feature = "dev_camera")]
        app.add_plugins(FreeCameraPlugin);

        app.add_plugins(CameraPlugin);
        app.add_plugins(MeshPickingPlugin);

        // Add our custom plugins
        app.add_plugins(EarthPlugin);
        app.add_plugins(MoonPlugin);
        app.add_plugins(CitiesPlugin);
        app.add_plugins(OrbitalPlugin);
        app.add_plugins(SatellitePlugin);
        app.add_plugins(TlePlugin);
        app.add_plugins(SpaceWeatherPlugin);
        app.add_plugins(LaunchLibraryPlugin);
        app.add_plugins(UiPlugin);
        app.add_plugins(SkyboxPlugin);
        app.add_plugins(VisualizationPlugin);
        app.add_plugins(LaunchesPlugin);
        app.add_plugins(GroundTrackPlugin);
        app.add_plugins(GroundTrackGizmoPlugin);
        app.add_plugins(HeatmapPlugin);
        app.add_systems(Startup, setup);
    }

    #[cfg(all(feature = "debug_basic_scene", not(feature = "debug_scene_camera")))]
    {
        app.add_systems(Startup, setup_basic_scene);
    }

    #[cfg(all(feature = "debug_scene_camera", not(feature = "debug_basic_scene")))]
    {
        app.add_plugins(CameraPlugin);
        app.add_plugins(bevy::picking::prelude::MeshPickingPlugin);
        app.add_plugins(visualization::SkyMaterialPlugin);
        app.add_plugins(SkyboxPlugin);
        app.add_systems(Startup, setup_debug_scene_camera);
        app.add_systems(Update, toggle_debug_scene_components);
    }

    #[cfg(all(
        feature = "dev_camera",
        any(
            all(
                not(feature = "debug_basic_scene"),
                not(feature = "debug_scene_camera")
            ),
            all(feature = "debug_basic_scene", feature = "debug_scene_camera")
        )
    ))]
    {
        app.add_systems(Startup, setup_dev_camera);
        app.add_systems(Update, toggle_dev_camera);
    }

    app.run();
}

#[cfg(all(
    test,
    feature = "dev_camera",
    any(
        all(
            not(feature = "debug_basic_scene"),
            not(feature = "debug_scene_camera")
        ),
        all(feature = "debug_basic_scene", feature = "debug_scene_camera")
    )
))]
mod dev_camera_tests {
    use super::*;

    #[test]
    fn f2_copies_pose_and_disables_inactive_free_camera_input() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, toggle_dev_camera);
        let pose = Transform::from_xyz(4_000.0, 2_000.0, 25_000.0).looking_at(Vec3::ZERO, Vec3::Y);
        let main = app
            .world_mut()
            .spawn((MainCamera, Camera::default(), pose))
            .id();
        let mut state = FreeCameraState::default();
        state.enabled = false;
        let dev = app
            .world_mut()
            .spawn((
                DevCamera,
                Camera {
                    is_active: false,
                    ..default()
                },
                Transform::default(),
                state,
            ))
            .id();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F2);
        app.update();
        assert!(!app.world().get::<Camera>(main).unwrap().is_active);
        assert!(app.world().get::<Camera>(dev).unwrap().is_active);
        assert!(app.world().get::<FreeCameraState>(dev).unwrap().enabled);
        assert_eq!(*app.world().get::<Transform>(dev).unwrap(), pose);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F2);
        app.update();
        assert!(app.world().get::<Camera>(main).unwrap().is_active);
        assert!(!app.world().get::<Camera>(dev).unwrap().is_active);
        assert!(!app.world().get::<FreeCameraState>(dev).unwrap().enabled);
    }
}
