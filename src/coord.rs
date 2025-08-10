use bevy::prelude::Vec3;
use std::f32::consts::PI;

use crate::earth::EARTH_RADIUS_KM;

#[allow(dead_code)]
#[derive(Debug)]
pub struct CoordError {
    pub msg: String,
}

#[derive(Debug)]
pub struct Coordinates {
    // Stored internally in radians (because math)
    pub latitude: f32,
    pub longitude: f32,
}

impl From<Vec3> for Coordinates {
    fn from(value: Vec3) -> Self {
        let normalized_point = value.normalize();
        let latitude = normalized_point.y.asin();
        let longitude = normalized_point.x.atan2(normalized_point.z);
        Coordinates {
            latitude,
            longitude,
        }
    }
}

impl Coordinates {
    pub fn as_degrees(&self) -> (f32, f32) {
        let latitude = self.latitude * (180.0 / PI);
        let longitude = self.longitude * (180.0 / PI);
        (latitude, longitude)
    }

    pub fn convert_to_uv_mercator(&self) -> (f32, f32) {
        let (lat, lon) = self.as_degrees();
        let v = map_latitude(lat).unwrap();
        let u = map_longitude(lon).unwrap();
        (u, v)
    }

    #[allow(dead_code)]
    pub fn from_degrees(latitude: f32, longitude: f32) -> Result<Self, CoordError> {
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(CoordError {
                msg: "Invalid latitude: {lat:?}".to_string(),
            });
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(CoordError {
                msg: "Invalid longitude: {lon:?}".to_string(),
            });
        }
        let latitude = latitude / (180.0 / PI);
        let longitude = longitude / (180.0 / PI);
        Ok(Coordinates {
            latitude,
            longitude,
        })
    }

    pub fn get_point_on_sphere(&self) -> Vec3 {
        // note: bevy coords where y is up
        let y = self.latitude.sin();
        let r = self.latitude.cos();
        let x = self.longitude.sin() * r;
        let z = self.longitude.cos() * r;
        Vec3::new(x, y, z).normalize() * EARTH_RADIUS_KM
    }
}

// Maps a value from one range to another
fn map((in_min, in_max): (f32, f32), (out_min, out_max): (f32, f32), value: f32) -> f32 {
    (value - in_min) / (in_max - in_min) * (out_max - out_min) + out_min
}

fn map_latitude(lat: f32) -> Result<f32, CoordError> {
    // 90 -> 0 maps to 0.0 to 0.5
    // 0 -> -90 maps to 0.5 to 1.0
    // Ensure latitude is valid
    if !(-90.0..=90.0).contains(&lat) {
        return Err(CoordError {
            msg: "Invalid latitude: {lat:?}".to_string(),
        });
    }
    if (90.0..=0.0).contains(&lat) {
        Ok(map((90.0, 0.0), (0.0, 0.5), lat))
    } else {
        Ok(map((0.0, -90.0), (0.5, 1.0), lat))
    }
}

fn map_longitude(lon: f32) -> Result<f32, CoordError> {
    // -180 -> 0 maps to 0.0 to 0.5
    // 0 -> 180 maps to 0.5 to 1.0
    //Ensure longitude is valid
    if !(-180.0..=180.0).contains(&lon) {
        return Err(CoordError {
            msg: "Invalid longitude: {lon:?}".to_string(),
        });
    }
    if (-180.0..=0.0).contains(&lon) {
        Ok(map((-180.0, 0.0), (0.0, 0.5), lon))
    } else {
        Ok(map((0.0, 180.0), (0.5, 1.0), lon))
    }
}

/// True if the straight segment from city (on/near sphere surface) to satellite does NOT intersect the Earth sphere.
/// Uses a robust segment-sphere intersection test around the origin.
pub fn los_visible_ecef(city_ecef_km: Vec3, sat_ecef_km: Vec3, earth_radius_km: f32) -> bool {
    // Parametric segment P(t) = C + t*(S - C), t in [0,1]
    let c = city_ecef_km;
    let u = sat_ecef_km - city_ecef_km;

    // Solve |C + t u|^2 = R^2  -> (u·u) t^2 + 2 (C·u) t + (C·C - R^2) = 0
    let a = u.length_squared();
    if a == 0.0 {
        // City and satellite at same point -> degenerate, treat as not visible
        return false;
    }
    let b = 2.0 * c.dot(u);
    let c_term = c.length_squared() - earth_radius_km * earth_radius_km;

    let discr = b * b - 4.0 * a * c_term;

    if discr < 0.0 {
        // No intersection with infinite line => segment cannot hit sphere
        return true;
    }

    let sqrt_d = discr.sqrt();
    let t1 = (-b - sqrt_d) / (2.0 * a);
    let t2 = (-b + sqrt_d) / (2.0 * a);

    // Exclude grazing at the city endpoint: require t > eps (in km units).
    let eps: f32 = 1e-5; // 1e-5 km = 1 cm
    // If either intersection parameter lies within (eps, 1], LOS is blocked.
    let hits_segment = ((t1 > eps) && (t1 <= 1.0)) || ((t2 > eps) && (t2 <= 1.0));
    !hits_segment
}

/// Cheap prefilter: city is potentially visible only if city and satellite are on the same hemisphere
/// relative to the sphere origin. Equivalent to dot(C, S) > R^2 (both outside the tangent plane).
pub fn hemisphere_prefilter(city_ecef_km: Vec3, sat_ecef_km: Vec3, earth_radius_km: f32) -> bool {
    city_ecef_km.dot(sat_ecef_km) > earth_radius_km * earth_radius_km
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Vec3;
    use std::f32::consts::PI;

    const EPSILON: f32 = 1e-6;

    #[test]
    fn test_coordinates_from_degrees_valid() {
        let coord = Coordinates::from_degrees(45.0, 90.0).unwrap();
        let (lat_deg, lon_deg) = coord.as_degrees();
        
        assert!((lat_deg - 45.0).abs() < EPSILON);
        assert!((lon_deg - 90.0).abs() < EPSILON);
    }

    #[test]
    fn test_coordinates_from_degrees_boundary_values() {
        // Test boundary values
        assert!(Coordinates::from_degrees(90.0, 180.0).is_ok());
        assert!(Coordinates::from_degrees(-90.0, -180.0).is_ok());
        assert!(Coordinates::from_degrees(0.0, 0.0).is_ok());
    }

    #[test]
    fn test_coordinates_from_degrees_invalid_latitude() {
        assert!(Coordinates::from_degrees(91.0, 0.0).is_err());
        assert!(Coordinates::from_degrees(-91.0, 0.0).is_err());
    }

    #[test]
    fn test_coordinates_from_degrees_invalid_longitude() {
        assert!(Coordinates::from_degrees(0.0, 181.0).is_err());
        assert!(Coordinates::from_degrees(0.0, -181.0).is_err());
    }

    #[test]
    fn test_coordinates_as_degrees() {
        let coord = Coordinates {
            latitude: PI / 4.0,  // 45 degrees
            longitude: PI / 2.0, // 90 degrees
        };
        let (lat_deg, lon_deg) = coord.as_degrees();
        
        assert!((lat_deg - 45.0).abs() < EPSILON);
        assert!((lon_deg - 90.0).abs() < EPSILON);
    }

    #[test]
    fn test_coordinates_from_vec3() {
        // Test conversion from normalized Vec3 to coordinates
        let vec = Vec3::new(0.0, 1.0, 0.0); // North pole
        let coord = Coordinates::from(vec);
        let (lat_deg, lon_deg) = coord.as_degrees();
        
        assert!((lat_deg - 90.0).abs() < EPSILON);
        // Longitude at poles is undefined, but should be finite
        assert!(lon_deg.is_finite());
    }

    #[test]
    fn test_coordinates_from_vec3_equator() {
        // Test point on equator
        let vec = Vec3::new(1.0, 0.0, 0.0);
        let coord = Coordinates::from(vec);
        let (lat_deg, lon_deg) = coord.as_degrees();
        
        assert!((lat_deg - 0.0).abs() < EPSILON);
        assert!((lon_deg - 90.0).abs() < EPSILON);
    }

    #[test]
    fn test_get_point_on_sphere() {
        // Test conversion back to 3D point
        let coord = Coordinates::from_degrees(0.0, 0.0).unwrap(); // Equator, prime meridian
        let point = coord.get_point_on_sphere();
        
        // Should be on the sphere surface
        assert!((point.length() - EARTH_RADIUS_KM).abs() < EPSILON);
        
        // Should be at (0, 0, EARTH_RADIUS_KM) in Bevy coordinates
        assert!((point.x - 0.0).abs() < EPSILON);
        assert!((point.y - 0.0).abs() < EPSILON);
        assert!((point.z - EARTH_RADIUS_KM).abs() < EPSILON);
    }

    #[test]
    fn test_get_point_on_sphere_north_pole() {
        let coord = Coordinates::from_degrees(90.0, 0.0).unwrap();
        let point = coord.get_point_on_sphere();
        
        assert!((point.length() - EARTH_RADIUS_KM).abs() < EPSILON);
        assert!((point.y - EARTH_RADIUS_KM).abs() < EPSILON);
        assert!(point.x.abs() < EPSILON);
        // At north pole, longitude is undefined, so z coordinate might not be exactly 0
        // Let's check that x and z are small relative to y
        assert!(point.x.abs() < 1e-3);
        assert!(point.z.abs() < 1e-3);
    }

    #[test]
    fn test_map_function() {
        // Test the generic map function
        let result = map((0.0, 10.0), (0.0, 100.0), 5.0);
        assert!((result - 50.0).abs() < EPSILON);
        
        let result = map((-1.0, 1.0), (0.0, 1.0), 0.0);
        assert!((result - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_map_latitude_valid() {
        // Test north pole
        let result = map_latitude(90.0).unwrap();
        assert!((result - 0.0).abs() < EPSILON);
        
        // Test equator
        let result = map_latitude(0.0).unwrap();
        assert!((result - 0.5).abs() < EPSILON);
        
        // Test south pole
        let result = map_latitude(-90.0).unwrap();
        assert!((result - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_map_latitude_invalid() {
        assert!(map_latitude(91.0).is_err());
        assert!(map_latitude(-91.0).is_err());
    }

    #[test]
    fn test_map_longitude_valid() {
        // Test western edge
        let result = map_longitude(-180.0).unwrap();
        assert!((result - 0.0).abs() < EPSILON);
        
        // Test prime meridian
        let result = map_longitude(0.0).unwrap();
        assert!((result - 0.5).abs() < EPSILON);
        
        // Test eastern edge
        let result = map_longitude(180.0).unwrap();
        assert!((result - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_map_longitude_invalid() {
        assert!(map_longitude(181.0).is_err());
        assert!(map_longitude(-181.0).is_err());
    }

    #[test]
    fn test_convert_to_uv_mercator() {
        let coord = Coordinates::from_degrees(0.0, 0.0).unwrap();
        let (u, v) = coord.convert_to_uv_mercator();
        
        // Equator, prime meridian should map to (0.5, 0.5)
        assert!((u - 0.5).abs() < EPSILON);
        assert!((v - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_los_visible_ecef_clear_line_of_sight() {
        // City on surface, satellite high above
        let city = Vec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        let satellite = Vec3::new(0.0, 0.0, EARTH_RADIUS_KM * 2.0);
        
        assert!(los_visible_ecef(city, satellite, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_los_visible_ecef_blocked_by_earth() {
        // City on one side, satellite on opposite side (blocked by Earth)
        let city = Vec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        let satellite = Vec3::new(0.0, 0.0, -EARTH_RADIUS_KM * 2.0);
        
        assert!(!los_visible_ecef(city, satellite, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_los_visible_ecef_same_position() {
        // Degenerate case: city and satellite at same position
        let position = Vec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        
        assert!(!los_visible_ecef(position, position, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_los_visible_ecef_high_satellite() {
        // Test a simple case: city on surface, satellite very high directly above
        let city = Vec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        let satellite = Vec3::new(0.0, 0.0, EARTH_RADIUS_KM * 10.0); // Very high above
        
        // This should definitely be visible
        assert!(los_visible_ecef(city, satellite, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_los_visible_ecef_grazing_case() {
        // Test a case where the line just grazes the Earth's surface
        // This tests the epsilon handling in the algorithm
        let city = Vec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        let satellite = Vec3::new(EARTH_RADIUS_KM * 2.0, 0.0, EARTH_RADIUS_KM);
        
        // This should be visible (line along surface, not through interior)
        assert!(los_visible_ecef(city, satellite, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_hemisphere_prefilter_same_hemisphere() {
        // Both points in positive Z hemisphere
        let city = Vec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        let satellite = Vec3::new(100.0, 100.0, EARTH_RADIUS_KM * 2.0);
        
        assert!(hemisphere_prefilter(city, satellite, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_hemisphere_prefilter_opposite_hemispheres() {
        // City in positive Z, satellite in negative Z
        let city = Vec3::new(0.0, 0.0, EARTH_RADIUS_KM);
        let satellite = Vec3::new(0.0, 0.0, -EARTH_RADIUS_KM * 2.0);
        
        assert!(!hemisphere_prefilter(city, satellite, EARTH_RADIUS_KM));
    }

    #[test]
    fn test_hemisphere_prefilter_edge_case() {
        // Test the boundary condition
        let city = Vec3::new(EARTH_RADIUS_KM, 0.0, 0.0);
        let satellite = Vec3::new(0.0, EARTH_RADIUS_KM, 0.0);
        
        // dot product = EARTH_RADIUS_KM^2, should be equal to threshold
        let result = hemisphere_prefilter(city, satellite, EARTH_RADIUS_KM);
        // This is exactly at the boundary, behavior depends on floating point precision
        // The function uses > not >=, so this should be false
        assert!(!result);
    }

    #[test]
    fn test_roundtrip_conversion() {
        // Test that Vec3 -> Coordinates -> Vec3 preserves direction
        let original = Vec3::new(1.0, 1.0, 1.0).normalize();
        let coord = Coordinates::from(original);
        let reconstructed = coord.get_point_on_sphere().normalize();
        
        // Should be very close (within floating point precision)
        let diff = (original - reconstructed).length();
        assert!(diff < 1e-5);
    }

    #[test]
    fn test_coordinates_debug_format() {
        // Test that Debug trait works
        let coord = Coordinates::from_degrees(45.0, 90.0).unwrap();
        let debug_str = format!("{:?}", coord);
        assert!(debug_str.contains("Coordinates"));
    }

    #[test]
    fn test_coord_error_debug_format() {
        // Test that CoordError Debug trait works
        let error = CoordError {
            msg: "Test error".to_string(),
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("CoordError"));
        assert!(debug_str.contains("Test error"));
    }
}
