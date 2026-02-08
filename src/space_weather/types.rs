//! Space weather data types and resources.

use bevy::prelude::*;
use chrono::{DateTime, TimeDelta, Utc};
use std::sync::{
    Arc, Mutex,
    mpsc::{Receiver, Sender},
};
use std::time::{Duration, Instant};

/// OVATION aurora forecasts are valid for 30-90 minutes.
/// We use 60 minutes as a conservative threshold.
pub const AURORA_FORECAST_VALIDITY: TimeDelta = TimeDelta::minutes(60);

#[derive(Resource, Clone, Debug)]
pub struct SpaceWeatherConfig {
    pub aurora_enabled: bool,
    pub aurora_alpha: f32,
    pub aurora_intensity_scale: f32,
    pub aurora_texture_width: u32,
    pub aurora_texture_height: u32,
    pub aurora_longitude_offset: f32,
    pub aurora_noise_strength: f32,
    pub aurora_noise_speed: f32,
    pub aurora_lat_start: f32,
    pub aurora_lat_end: f32,
    pub ovation_refresh: Duration,
    pub kp_refresh: Duration,
    pub solar_wind_refresh: Duration,
}

impl Default for SpaceWeatherConfig {
    fn default() -> Self {
        Self {
            aurora_enabled: true,
            aurora_alpha: 0.6,
            aurora_intensity_scale: 1.0,
            aurora_texture_width: 256,
            aurora_texture_height: 128,
            // Longitude offset to convert NOAA OVATION AACGM magnetic coordinates to geographic.
            // Empirically determined (-149° as of 2026) by comparison with NASA SWPC plots.
            // May need adjustment over time as magnetic pole drifts (~50-60 km/year).
            aurora_longitude_offset: -149.0,
            aurora_noise_strength: 0.4,
            aurora_noise_speed: 0.002,
            aurora_lat_start: 45.0,
            aurora_lat_end: 65.0,
            ovation_refresh: Duration::from_secs(600),
            kp_refresh: Duration::from_secs(900),
            solar_wind_refresh: Duration::from_secs(120),
        }
    }
}

#[derive(Resource)]
pub struct SpaceWeatherState {
    pub last_ovation_request: Instant,
    pub last_kp_request: Instant,
    pub last_mag_request: Instant,
    pub last_plasma_request: Instant,
    pub ovation_error: Option<String>,
    pub kp_error: Option<String>,
    pub mag_error: Option<String>,
    pub plasma_error: Option<String>,
}

impl Default for SpaceWeatherState {
    fn default() -> Self {
        let now = Instant::now() - Duration::from_secs(3600);
        Self {
            last_ovation_request: now,
            last_kp_request: now,
            last_mag_request: now,
            last_plasma_request: now,
            ovation_error: None,
            kp_error: None,
            mag_error: None,
            plasma_error: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AuroraPoint {
    pub lat: f32,
    pub lon: f32,
    pub value: f32,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct AuroraGrid {
    pub points: Vec<AuroraPoint>,
    pub grid_values: Vec<f32>,
    pub grid_width: usize,
    pub grid_height: usize,
    pub lon_min: f32,
    pub lat_min: f32,
    pub lon_step: f32,
    pub lat_step: f32,
    pub max_value: f32,
    pub updated_utc: Option<DateTime<Utc>>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct KpIndex {
    pub value: Option<f32>,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Resource, Default, Clone)]
pub struct SolarWind {
    pub bt: Option<f32>,
    pub bz: Option<f32>,
    pub speed: Option<f32>,
    pub density: Option<f32>,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy)]
pub enum SpaceWeatherFeed {
    Ovation,
    Kp,
    Mag,
    Plasma,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum SpaceWeatherCommand {
    FetchOvation,
    FetchKp,
    FetchMag,
    FetchPlasma,
}

#[derive(Debug)]
pub enum SpaceWeatherResult {
    Ovation {
        grid: AuroraGrid,
    },
    Kp {
        kp: KpIndex,
    },
    Mag {
        bt: Option<f32>,
        bz: Option<f32>,
        timestamp: Option<DateTime<Utc>>,
    },
    Plasma {
        speed: Option<f32>,
        density: Option<f32>,
        timestamp: Option<DateTime<Utc>>,
    },
    Error {
        feed: SpaceWeatherFeed,
        error: String,
    },
}

#[derive(Resource)]
pub struct SpaceWeatherChannels {
    pub cmd_tx: Sender<SpaceWeatherCommand>,
    pub res_rx: Arc<Mutex<Receiver<SpaceWeatherResult>>>,
}
