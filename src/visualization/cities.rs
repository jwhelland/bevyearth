use crate::core::coordinates::Coordinates;
use bevy::prelude::*;
use bevy::render::mesh::SphereKind;
use bevy::render::mesh::SphereMeshBuilder;

/// Plugin for city visualization and management
pub struct CitiesPlugin;

impl Plugin for CitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (initialize_cities_ecef, spawn_city_markers).chain(),
        );
    }
}

/// Initialize the CitiesEcef resource with actual city data
fn initialize_cities_ecef(mut commands: Commands) {
    let major_cities = major_cities_data();
    let mut cache = Vec::with_capacity(major_cities.len());

    for (_name, latitude, longitude) in &major_cities {
        let ecef = Coordinates::from_degrees(*latitude, *longitude)
            .unwrap()
            .get_point_on_sphere(); // already returns EARTH_RADIUS_KM scaled Vec3
        cache.push(ecef);
    }

    commands.insert_resource(CitiesEcef(cache));
}

const CITY_RADIUS: f32 = 15.0;

// CPU cache of city locations in ECEF kilometers
#[derive(Resource, Deref, DerefMut, Default)]
pub struct CitiesEcef(pub Vec<Vec3>);

// Create a component to store city information.
// Not used in this example, but could be used for a tooltip or similar.
#[allow(dead_code)]
#[derive(Component)]
pub struct CityMarker {
    pub name: String,
}

// Expose major_cities so both mesh spawning and ECEF cache use the same data
pub fn major_cities_data() -> Vec<(String, f32, f32)> {
    vec![
        (String::from("Tokyo"), 35.6762, 139.6503),
        (String::from("Delhi"), 28.6139, 77.2090),
        (String::from("Shanghai"), 31.2304, 121.4737),
        (String::from("São Paulo"), -23.5505, -46.6333),
        (String::from("Mexico City"), 19.4326, -99.1332),
        (String::from("Cairo"), 30.0444, 31.2357),
        (String::from("Mumbai"), 19.0760, 72.8777),
        (String::from("Beijing"), 39.9042, 116.4074),
        (String::from("Dhaka"), 23.8103, 90.4125),
        (String::from("Osaka"), 34.6937, 135.5023),
        (String::from("New York"), 40.7128, -74.0060),
        (String::from("Karachi"), 24.8607, 67.0011),
        (String::from("Buenos Aires"), -34.6037, -58.3816),
        (String::from("Istanbul"), 41.0082, 28.9784),
        (String::from("Kolkata"), 22.5726, 88.3639),
        (String::from("Lagos"), 6.5244, 3.3792),
        (String::from("London"), 51.5074, -0.1278),
        (String::from("Los Angeles"), 34.0522, -118.2437),
        (String::from("Manila"), 14.5995, 120.9842),
        (String::from("Rio de Janeiro"), -22.9068, -43.1729),
        (String::from("Tianjin"), 39.3434, 117.3616),
        (String::from("Kinshasa"), -4.4419, 15.2663),
        (String::from("Paris"), 48.8566, 2.3522),
        (String::from("Shenzhen"), 22.5431, 114.0579),
        (String::from("Jakarta"), -6.2088, 106.8456),
        (String::from("Bangalore"), 12.9716, 77.5946),
        (String::from("Moscow"), 55.7558, 37.6173),
        (String::from("Chennai"), 13.0827, 80.2707),
        (String::from("Lima"), -12.0464, -77.0428),
        (String::from("Bangkok"), 13.7563, 100.5018),
        (String::from("Seoul"), 37.5665, 126.978),
        (String::from("Hyderabad"), 17.3850, 78.4867),
        (String::from("Chengdu"), 30.5728, 104.0668),
        (String::from("Singapore"), 1.3521, 103.8198),
        (String::from("Ho Chi Minh City"), 10.8231, 106.6297),
        (String::from("Toronto"), 43.6532, -79.3832),
        (String::from("Sydney"), -33.8688, 151.2093),
        (String::from("Johannesburg"), -26.2041, 28.0473),
        (String::from("Chicago"), 41.8781, -87.6298),
        (String::from("Taipei"), 25.0330, 121.5654),
    ]
}

// Startup system: spawn city visual markers
pub fn spawn_city_markers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let major_cities = major_cities_data();

    // Visual markers
    let sphere_mesh = SphereMeshBuilder::new(1.0, SphereKind::Ico { subdivisions: 32 });
    for (name, latitude, longitude) in major_cities {
        let coords = Coordinates::from_degrees(latitude, longitude)
            .unwrap()
            .get_point_on_sphere();

        commands.spawn((
            Mesh3d(meshes.add(sphere_mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 0.0),
                unlit: true,
                ..default()
            })),
            Transform::from_translation(coords).with_scale(Vec3::splat(CITY_RADIUS)),
            CityMarker { name },
        ));
    }
}
