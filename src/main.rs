// Inspired by https://blog.graysonhead.net/posts/bevy-proc-earth-1/

use bevy::picking::prelude::*;
use bevy::render::mesh::Mesh;
use bevy::render::view::RenderLayers;
use bevy::{prelude::*, render::camera::Viewport, window::PrimaryWindow};

use bevy_egui::{
    EguiContext, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass,
    PrimaryEguiContext, egui,
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

mod cities;
mod coord;
mod earth;
use crate::earth::EARTH_RADIUS_KM;
use cities::{CitiesEcef, spawn_city_population_spheres};
use coord::{hemisphere_prefilter, los_visible_ecef};
use earth::generate_faces;

#[derive(Component)]
struct Satellite;

// Arrow rendering config
#[derive(Resource)]
struct ArrowConfig {
    enabled: bool,
    color: Color,
    max_visible: usize,
    lift_m: f32,       // lift city endpoint off the surface (meters)
    tip_offset_m: f32, // offset before satellite tip (meters)
    head_len_pct: f32,
    head_min_m: f32,
    head_max_m: f32,
    head_radius_pct: f32,
    shaft_len_pct: f32,   // fraction of city->sat distance to draw as shaft
    shaft_min_m: f32,     // minimum shaft length in meters
    shaft_max_m: f32,     // maximum shaft length in meters
}
impl Default for ArrowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            color: Color::srgb(0.1, 0.9, 0.3),
            max_visible: 200,
            lift_m: 1000.0,
            tip_offset_m: 2000.0,
            head_len_pct: 0.02,
            head_min_m: 10_000.0,
            head_max_m: 100_000.0,
            head_radius_pct: 0.4,
            shaft_len_pct: 0.12,  // draw only the first 12% toward satellite
            shaft_min_m: 5_000.0,
            shaft_max_m: 400_000.0,
        }
    }
}

// Satellite ECEF position resource (in kilometers to match EARTH_RADIUS_KM)
#[derive(Resource, Deref, DerefMut, Default)]
struct SatEcef(pub Vec3);

#[derive(Resource)]
struct UIState {
    name: String,
}
impl Default for UIState {
    fn default() -> Self {
        Self {
            name: "".to_string(),
        }
    }
}
// The `ShowAxes` component is attached to an entity to get the `draw_axes` system to
// display axes according to its Transform component.
#[derive(Component)]
struct ShowAxes;

// System: update satellite ECEF resource from Satellite entity transform
fn update_satellite_ecef(
    sat_query: Query<&Transform, With<Satellite>>,
    mut sat_res: ResMut<SatEcef>,
) {
    if let Ok(t) = sat_query.single() {
        // World coordinates are already in km scale in this app (EARTH_RADIUS_KM sphere)
        sat_res.0 = t.translation;
    }
}

// System: compute visibility and draw arrows using Gizmos for now (prototype of geometry path).
// We will draw lines from city -> satellite and a small cone approximation at the satellite pointing toward the city.
fn draw_city_to_satellite_arrows(
    mut gizmos: Gizmos,
    sat: Res<SatEcef>,
    cities: Option<Res<CitiesEcef>>,
    config: Res<ArrowConfig>,
) {
    if !config.enabled {
        return;
    }
    let Some(cities) = cities else {
        return;
    };

    // constants conversion meters->kilometers
    let lift_km = config.lift_m / 1000.0;
    let tip_offset_km = config.tip_offset_m / 1000.0;
    let head_min_km = config.head_min_m / 1000.0;
    let head_max_km = config.head_max_m / 1000.0;

    let sat_pos = **sat;
    let mut drawn = 0usize;
    for &city in cities.iter() {
        if !hemisphere_prefilter(city, sat_pos, EARTH_RADIUS_KM) {
            continue;
        }
        if !los_visible_ecef(city, sat_pos, EARTH_RADIUS_KM) {
            continue;
        }
        // Direction and lifted city endpoint to avoid z-fighting with globe
        let dir = (sat_pos - city).normalize();
        let city_lifted = city.normalize() * (EARTH_RADIUS_KM + lift_km);

        // Compute total city->sat distance from the lifted point
        let total_len = (sat_pos - city_lifted).length();

        // Compute a short shaft length near the city only
        let mut shaft_len = config.shaft_len_pct * total_len;
        // clamp by mins/max (convert meters->km)
        let shaft_min_km = config.shaft_min_m / 1000.0;
        let shaft_max_km = config.shaft_max_m / 1000.0;
        shaft_len = shaft_len.clamp(shaft_min_km, shaft_max_km).min(total_len * 0.9);

        // Shaft end point along direction toward satellite
        let shaft_end = city_lifted + dir * shaft_len;

        // Draw only a short shaft near the city that points toward the satellite
        gizmos.line(shaft_end, city_lifted, config.color);

        // Arrowhead: draw at the end of the shaft (near the city), pointing toward the satellite
        let head_len = (config.head_len_pct * total_len).clamp(head_min_km, head_max_km).min(shaft_len * 0.8);
        let tip_pos = shaft_end;              // tip at end of shaft
        let base = tip_pos - dir * head_len;  // base moved back toward city

        // Build orthonormal frame
        let up = dir.any_orthonormal_vector();
        let right = dir.cross(up).normalize();
        let radius = head_len * config.head_radius_pct;
        let a = base + up * radius;
        let b = base - up * radius * 0.5 + right * radius * 0.8660254;
        let c = base - up * radius * 0.5 - right * radius * 0.8660254;

        // Tip-connected edges
        gizmos.line(a, tip_pos, config.color);
        gizmos.line(b, tip_pos, config.color);
        gizmos.line(c, tip_pos, config.color);
        // Base triangle for visual stability
        gizmos.line(b, a, config.color);
        gizmos.line(c, b, config.color);
        gizmos.line(a, c, config.color);

        drawn += 1;
        if drawn >= config.max_visible {
            break;
        }
    }
}

fn draw_axes(mut gizmos: Gizmos, query: Query<&Transform, With<ShowAxes>>) {
    for &transform in &query {
        gizmos.axes(transform, 8000.0);
    }
}

pub fn setup(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // asset_server: Res<AssetServer>,
) {
    // Disable the automatic creation of a primary context to set it up manually for the camera we need.
    egui_global_settings.auto_create_primary_context = false;

    // Small sphere at origin to show axes
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(500.0).mesh().ico(5).unwrap())),
        MeshMaterial3d(materials.add(Color::srgb(1., 0., 0.))),
        Satellite,
        Transform::from_xyz(25000., 0., 0.),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap())),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0., 0.))),
        ShowAxes,
    ));
    commands.spawn((
        PanOrbitCamera::default(),
        Transform::from_xyz(25000.0, 8.0, 4.0),
    ));
    commands.spawn((
        Camera2d,
        PrimaryEguiContext,
        RenderLayers::none(),
        Camera {
            order: 1,
            ..default()
        },
        Transform::from_xyz(25000.0, 8.0, 4.0),
    ));
}

// This function runs every frame. Therefore, updating the viewport after drawing the gui.
// With a resource which stores the dimensions of the panels, the update of the Viewport can
// be done in another system.
fn ui_example_system(
    mut contexts: EguiContexts,
    mut camera: Single<&mut Camera, Without<EguiContext>>,
    window: Single<&mut Window, With<PrimaryWindow>>,
    mut state: ResMut<UIState>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut left = egui::SidePanel::left("left_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Left resizeable panel");
            // ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut state.name);
            });
        })
        .response
        .rect
        .width(); // height is ignored, as the panel has a hight of 100% of the screen

    let mut right = egui::SidePanel::right("right_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Right resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .width(); // height is ignored, as the panel has a height of 100% of the screen

    let mut top = egui::TopBottomPanel::top("top_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Top resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .height(); // width is ignored, as the panel has a width of 100% of the screen
    let mut bottom = egui::TopBottomPanel::bottom("bottom_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Bottom resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .height(); // width is ignored, as the panel has a width of 100% of the screen

    // Scale from logical units to physical units.
    left *= window.scale_factor();
    right *= window.scale_factor();
    top *= window.scale_factor();
    bottom *= window.scale_factor();

    // -------------------------------------------------
    // |  left   |            top   ^^^^^^   |  right  |
    // |  panel  |           panel  height   |  panel  |
    // |         |                  vvvvvv   |         |
    // |         |---------------------------|         |
    // |         |                           |         |
    // |<-width->|          viewport         |<-width->|
    // |         |                           |         |
    // |         |---------------------------|         |
    // |         |          bottom   ^^^^^^  |         |
    // |         |          panel    height  |         |
    // |         |                   vvvvvv  |         |
    // -------------------------------------------------
    //
    // The upper left point of the viewport is the width of the left panel and the height of the
    // top panel
    //
    // The width of the viewport the width of the top/bottom panel
    // Alternative the width can be calculated as follow:
    // size.x = window width - left panel width - right panel width
    //
    // The height of the viewport is:
    // size.y = window height - top panel height - bottom panel height
    //
    // Therefore we use the alternative for the width, as we can callculate the Viewport as
    // following:

    let pos = UVec2::new(left as u32, top as u32);
    let size = UVec2::new(window.physical_width(), window.physical_height())
        - pos
        - UVec2::new(right as u32, bottom as u32);

    camera.viewport = Some(Viewport {
        physical_position: pos,
        physical_size: size,
        ..default()
    });

    Ok(())
}

fn main() {
    App::new()
        .init_resource::<UIState>()
        .init_resource::<ArrowConfig>()
        .init_resource::<SatEcef>()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(MeshPickingPlugin)
        .add_systems(
            Startup,
            (setup, generate_faces, spawn_city_population_spheres).chain(),
        )
        .add_systems(
            Update,
            (
                draw_axes.after(setup),
                update_satellite_ecef,
                draw_city_to_satellite_arrows.after(update_satellite_ecef),
            ),
        )
        .add_systems(EguiPrimaryContextPass, ui_example_system)
        .run();
}
