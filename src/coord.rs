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

    // If either intersection parameter lies within the segment [0,1], LOS is blocked.
    let hits_segment = (0.0..=1.0).contains(&t1) || (0.0..=1.0).contains(&t2);
    !hits_segment
}

/// Cheap prefilter: city is potentially visible only if city and satellite are on the same hemisphere
/// relative to the sphere origin. Equivalent to dot(C, S) > R^2 (both outside the tangent plane).
pub fn hemisphere_prefilter(city_ecef_km: Vec3, sat_ecef_km: Vec3, earth_radius_km: f32) -> bool {
    city_ecef_km.dot(sat_ecef_km) > earth_radius_km * earth_radius_km
}
