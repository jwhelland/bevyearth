//! Space weather systems for polling, parsing, and rendering.

use bevy::asset::RenderAssetUsages;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use chrono::{DateTime, Utc};
use std::cmp::Ordering;
use std::time::Instant;

use crate::core::coordinates::Coordinates;
use crate::orbital::time::SimulationTime;
use crate::space_weather::fetcher::start_space_weather_worker;
use crate::space_weather::types::{
    AuroraGrid, KpIndex, SolarWind, SpaceWeatherChannels, SpaceWeatherCommand, SpaceWeatherConfig,
    SpaceWeatherFeed, SpaceWeatherResult, SpaceWeatherState, AURORA_FORECAST_VALIDITY,
};
use crate::visualization::earth::EarthMeshHandle;

#[derive(Resource, Default)]
pub(crate) struct AuroraRenderState {
    pub image_handle: Option<Handle<Image>>,
    pub material_handle: Option<Handle<StandardMaterial>>,
    pub entity: Option<Entity>,
    pub width: u32,
    pub height: u32,
    pub intensity_buffer: Vec<f32>,
}

pub fn setup_space_weather_worker(mut commands: Commands) {
    let channels = start_space_weather_worker();
    println!("[INIT] Space weather worker started");
    commands.insert_resource(channels);
}

pub fn poll_space_weather(
    config: Res<SpaceWeatherConfig>,
    mut state: ResMut<SpaceWeatherState>,
    channels: Option<Res<SpaceWeatherChannels>>,
) {
    let Some(channels) = channels else { return };
    let now = Instant::now();

    if now.duration_since(state.last_ovation_request) >= config.ovation_refresh {
        state.last_ovation_request = now;
        let _ = channels.cmd_tx.send(SpaceWeatherCommand::FetchOvation);
    }

    if now.duration_since(state.last_kp_request) >= config.kp_refresh {
        state.last_kp_request = now;
        let _ = channels.cmd_tx.send(SpaceWeatherCommand::FetchKp);
    }

    if now.duration_since(state.last_mag_request) >= config.solar_wind_refresh {
        state.last_mag_request = now;
        let _ = channels.cmd_tx.send(SpaceWeatherCommand::FetchMag);
    }

    if now.duration_since(state.last_plasma_request) >= config.solar_wind_refresh {
        state.last_plasma_request = now;
        let _ = channels.cmd_tx.send(SpaceWeatherCommand::FetchPlasma);
    }
}

pub fn apply_space_weather_results(
    mut aurora: ResMut<AuroraGrid>,
    mut kp: ResMut<KpIndex>,
    mut solar_wind: ResMut<SolarWind>,
    mut state: ResMut<SpaceWeatherState>,
    channels: Option<Res<SpaceWeatherChannels>>,
    mut ovation_logged: Local<bool>,
) {
    let Some(channels) = channels else { return };
    let Ok(guard) = channels.res_rx.lock() else {
        return;
    };

    while let Ok(msg) = guard.try_recv() {
        match msg {
            SpaceWeatherResult::Ovation { grid } => {
                if !*ovation_logged
                    && (grid.grid_width > 0 || !grid.points.is_empty() || !grid.grid_values.is_empty())
                {
                    println!(
                        "[OVATION] received grid={}x{} values={} max={:.3}",
                        grid.grid_width,
                        grid.grid_height,
                        grid.grid_values.len(),
                        grid.max_value
                    );
                    *ovation_logged = true;
                }
                *aurora = grid;
                state.ovation_error = None;
            }
            SpaceWeatherResult::Kp { kp: kp_data } => {
                *kp = kp_data;
                state.kp_error = None;
            }
            SpaceWeatherResult::Mag { bt, bz, timestamp } => {
                solar_wind.bt = bt;
                solar_wind.bz = bz;
                update_timestamp(&mut solar_wind.timestamp, timestamp);
                state.mag_error = None;
            }
            SpaceWeatherResult::Plasma {
                speed,
                density,
                timestamp,
            } => {
                solar_wind.speed = speed;
                solar_wind.density = density;
                update_timestamp(&mut solar_wind.timestamp, timestamp);
                state.plasma_error = None;
            }
            SpaceWeatherResult::Error { feed, error } => {
                match feed {
                    SpaceWeatherFeed::Ovation => state.ovation_error = Some(error),
                    SpaceWeatherFeed::Kp => state.kp_error = Some(error),
                    SpaceWeatherFeed::Mag => state.mag_error = Some(error),
                    SpaceWeatherFeed::Plasma => state.plasma_error = Some(error),
                }
            }
        }
    }
}

pub fn initialize_aurora_overlay(
    mut commands: Commands,
    earth_mesh: Option<Res<EarthMeshHandle>>,
    config: Res<SpaceWeatherConfig>,
    mut render_state: ResMut<AuroraRenderState>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if render_state.entity.is_some() {
        return;
    }
    let Some(earth_mesh) = earth_mesh else { return };

    let width = config.aurora_texture_width.max(8);
    let height = config.aurora_texture_height.max(4);
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0u8; 4],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let image_handle = images.add(image);

    let material_handle = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 1.0),
        base_color_texture: Some(image_handle.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        depth_bias: 1.0,
        ..default()
    });

    let visibility = if config.aurora_enabled {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    let entity = commands
        .spawn((
            Mesh3d(earth_mesh.handle.clone()),
            MeshMaterial3d(material_handle.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(1.002)),
            visibility,
            Pickable::IGNORE,
            Name::new("Aurora Overlay"),
        ))
        .id();

    render_state.image_handle = Some(image_handle);
    render_state.material_handle = Some(material_handle);
    render_state.entity = Some(entity);
    render_state.width = width;
    render_state.height = height;
    render_state.intensity_buffer = vec![0.0; (width * height) as usize];
}

pub fn update_aurora_texture(
    config: Res<SpaceWeatherConfig>,
    aurora: Res<AuroraGrid>,
    mut render_state: ResMut<AuroraRenderState>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut debug_logged: Local<bool>,
) {
    if !aurora.is_changed() && !config.is_changed() {
        return;
    }
    let Some(image_handle) = render_state.image_handle.clone() else {
        return;
    };
    let Some(image) = images.get_mut(&image_handle) else {
        return;
    };

    let width = render_state.width as usize;
    let height = render_state.height as usize;
    if width == 0 || height == 0 {
        return;
    }

    if render_state.intensity_buffer.len() != width * height {
        render_state.intensity_buffer = vec![0.0; width * height];
    } else {
        for v in render_state.intensity_buffer.iter_mut() {
            *v = 0.0;
        }
    }

    let max_value = aurora.max_value.max(1.0);

    if !aurora.grid_values.is_empty()
        && aurora.grid_width > 0
        && aurora.grid_height > 0
        && aurora.lon_step.abs() > f32::EPSILON
        && aurora.lat_step.abs() > f32::EPSILON
    {
        for y in 0..aurora.grid_height {
            let lat = aurora.lat_min + (y as f32 * aurora.lat_step);
            for x in 0..aurora.grid_width {
                let idx = y * aurora.grid_width + x;
                let value = aurora.grid_values[idx] * aurora_lat_mask(lat);
                if value <= 0.0 {
                    continue;
                }
                let mut lon = aurora.lon_min + (x as f32 * aurora.lon_step);
                // Apply longitude offset for magnetic->geographic coordinate conversion
                lon += config.aurora_longitude_offset;
                // Normalize to -180..180
                while lon > 180.0 {
                    lon -= 360.0;
                }
                while lon < -180.0 {
                    lon += 360.0;
                }
                let Ok(coords) = Coordinates::from_degrees(lat, lon) else {
                    continue;
                };
                let (u, v) = coords.convert_to_uv_mercator();
                let px = (u * (render_state.width as f32 - 1.0))
                    .round()
                    .clamp(0.0, render_state.width as f32 - 1.0) as usize;
                let py = (v * (render_state.height as f32 - 1.0))
                    .round()
                    .clamp(0.0, render_state.height as f32 - 1.0) as usize;
                let out_idx = py * width + px;
                if value > render_state.intensity_buffer[out_idx] {
                    render_state.intensity_buffer[out_idx] = value;
                }
            }
        }
    } else {
        for point in aurora.points.iter() {
            let mut lon = point.lon;
            // Apply longitude offset for magnetic->geographic coordinate conversion
            lon += config.aurora_longitude_offset;
            // Normalize to -180..180
            while lon > 180.0 {
                lon -= 360.0;
            }
            while lon < -180.0 {
                lon += 360.0;
            }

            let Ok(coords) = Coordinates::from_degrees(point.lat, lon) else {
                continue;
            };
            let (u, v) = coords.convert_to_uv_mercator();
            let x = (u * (render_state.width as f32 - 1.0))
                .round()
                .clamp(0.0, render_state.width as f32 - 1.0) as usize;
            let y = (v * (render_state.height as f32 - 1.0))
                .round()
                .clamp(0.0, render_state.height as f32 - 1.0) as usize;
            let idx = y * width + x;
            let value = point.value * aurora_lat_mask(point.lat);
            if value > render_state.intensity_buffer[idx] {
                render_state.intensity_buffer[idx] = value;
            }
        }
    }

    let has_data = !aurora.grid_values.is_empty() || !aurora.points.is_empty();
    if aurora.is_changed() && has_data && !*debug_logged {
        let grid_nonzero = aurora.grid_values.iter().filter(|v| **v > 0.0).count();
        let mut buffer_nonzero = 0usize;
        let mut buffer_max = 0.0_f32;
        for value in render_state.intensity_buffer.iter() {
            if *value > 0.0 {
                buffer_nonzero += 1;
            }
            if *value > buffer_max {
                buffer_max = *value;
            }
        }
        println!(
            "[AURORA] grid={}x{} lon_min={:.2} lon_step={:.2} lat_min={:.2} lat_step={:.2} max={:.3} grid_nonzero={} buffer_nonzero={} buffer_max={:.3} points={} alpha={:.2} intensity_scale={:.2} enabled={}",
            aurora.grid_width,
            aurora.grid_height,
            aurora.lon_min,
            aurora.lon_step,
            aurora.lat_min,
            aurora.lat_step,
            aurora.max_value,
            grid_nonzero,
            buffer_nonzero,
            buffer_max,
            aurora.points.len(),
            config.aurora_alpha,
            config.aurora_intensity_scale,
            config.aurora_enabled
        );
        *debug_logged = true;
    }

    let data_len = width * height * 4;
    let data = image.data.get_or_insert_with(|| vec![0; data_len]);
    if data.len() != data_len {
        data.resize(data_len, 0);
    }

    // Floor: skip aurora values below 10% of max to filter noise
    let floor = max_value * 0.1;

    for (i, chunk) in data.chunks_exact_mut(4).enumerate() {
        let raw = render_state.intensity_buffer[i];
        if raw <= floor {
            chunk[0] = 0;
            chunk[1] = 0;
            chunk[2] = 0;
            chunk[3] = 0;
            continue;
        }
        // Normalize, remapping floor..max to 0..1
        let normalized = ((raw - floor) / (max_value - floor)).clamp(0.0, 1.0);
        let boosted = (normalized * config.aurora_intensity_scale).min(1.0);
        // Color encodes both hue and natural brightness
        let color = aurora_color(boosted);
        // Alpha uniformly scales additive brightness
        let alpha = config.aurora_alpha;
        chunk[0] = (color[0] * alpha * 255.0).clamp(0.0, 255.0) as u8;
        chunk[1] = (color[1] * alpha * 255.0).clamp(0.0, 255.0) as u8;
        chunk[2] = (color[2] * alpha * 255.0).clamp(0.0, 255.0) as u8;
        chunk[3] = (alpha * 255.0).clamp(0.0, 255.0) as u8;
    }

    // Workaround for Bevy runtime-created asset change detection (bevyengine/bevy#17220):
    // touching the material forces it to re-check its dependent textures on the GPU.
    if let Some(ref mat_handle) = render_state.material_handle {
        materials.get_mut(mat_handle);
    }
}

pub fn sync_aurora_visibility(
    config: Res<SpaceWeatherConfig>,
    aurora: Res<AuroraGrid>,
    sim_time: Res<SimulationTime>,
    render_state: Res<AuroraRenderState>,
    mut visibility: Query<&mut Visibility>,
) {
    if !config.is_changed() && !aurora.is_changed() && !sim_time.is_changed() {
        return;
    }
    let Some(entity) = render_state.entity else {
        return;
    };
    if let Ok(mut vis) = visibility.get_mut(entity) {
        let has_data = !aurora.grid_values.is_empty() || !aurora.points.is_empty();

        // Check if forecast is still valid relative to simulation time
        let is_valid = if let Some(forecast_time) = aurora.updated_utc {
            sim_time.current_utc <= forecast_time + AURORA_FORECAST_VALIDITY
        } else {
            false // No timestamp = hide by default
        };

        let should_show = config.aurora_enabled && has_data && config.aurora_alpha > 0.0 && is_valid;
        *vis = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_timestamp(current: &mut Option<DateTime<Utc>>, incoming: Option<DateTime<Utc>>) {
    let Some(incoming) = incoming else { return };
    match current {
        Some(existing) => {
            if incoming > *existing {
                *current = Some(incoming);
            }
        }
        None => {
            *current = Some(incoming);
        }
    }
}

fn aurora_color(t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    // Dim green → bright green → bright magenta
    let (start, mid, end) = ([0.05, 0.35, 0.12], [0.2, 1.0, 0.4], [1.0, 0.5, 0.9]);
    if t <= 0.5 {
        let p = t / 0.5;
        lerp_color(start, mid, p)
    } else {
        let p = (t - 0.5) / 0.5;
        lerp_color(mid, end, p)
    }
}

fn lerp_color(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn aurora_lat_mask(lat: f32) -> f32 {
    let abs_lat = lat.abs();
    let start = 40.0;
    let end = 60.0;
    if abs_lat <= start {
        0.0
    } else if abs_lat >= end {
        1.0
    } else {
        let t = ((abs_lat - start) / (end - start)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
}

#[allow(dead_code)]
fn percentile_cutoff(values: &[f32], percentile: f32, max_value: f32) -> f32 {
    if values.is_empty() || max_value <= 0.0 {
        return 0.0;
    }
    let mut samples: Vec<f32> = values.iter().copied().filter(|v| *v > 0.0).collect();
    if samples.len() < 32 {
        return 0.0;
    }
    samples.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let p = percentile.clamp(0.0, 1.0);
    let idx = ((samples.len() - 1) as f32 * p).round() as usize;
    let cutoff = samples[idx].min(max_value * 0.95);
    if cutoff.is_finite() { cutoff } else { 0.0 }
}
