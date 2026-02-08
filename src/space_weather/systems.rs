//! Space weather systems for polling, parsing, and rendering.

use bevy::asset::RenderAssetUsages;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use chrono::{DateTime, Utc};
use std::cmp::Ordering;
use std::time::Instant;

use crate::core::coordinates::Coordinates;
use crate::core::space::ecef_to_bevy_km;
use crate::orbital::{SimulationTime, SunDirection};
use crate::space_weather::fetcher::start_space_weather_worker;
use crate::space_weather::types::{
    AURORA_FORECAST_VALIDITY, AuroraGrid, KpIndex, SolarWind, SpaceWeatherChannels,
    SpaceWeatherCommand, SpaceWeatherConfig, SpaceWeatherFeed, SpaceWeatherResult,
    SpaceWeatherState,
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
    pub noise_map: Vec<f32>,
    pub noise_width: usize,
    pub noise_height: usize,
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
                    && (grid.grid_width > 0
                        || !grid.points.is_empty()
                        || !grid.grid_values.is_empty())
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
            SpaceWeatherResult::Error { feed, error } => match feed {
                SpaceWeatherFeed::Ovation => state.ovation_error = Some(error),
                SpaceWeatherFeed::Kp => state.kp_error = Some(error),
                SpaceWeatherFeed::Mag => state.mag_error = Some(error),
                SpaceWeatherFeed::Plasma => state.plasma_error = Some(error),
            },
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
        base_color: Color::BLACK,
        base_color_texture: None,
        emissive: LinearRgba::rgb(4.0, 4.0, 4.0),
        emissive_texture: Some(image_handle.clone()),
        metallic: 0.0,
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        unlit: false,
        alpha_mode: AlphaMode::Add,
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
    render_state.noise_width = 128;
    render_state.noise_height = 64;
    render_state.noise_map =
        generate_noise_map(render_state.noise_width, render_state.noise_height);
}

#[allow(clippy::too_many_arguments)]
pub fn update_aurora_texture(
    config: Res<SpaceWeatherConfig>,
    aurora: Res<AuroraGrid>,
    sun_direction: Res<SunDirection>,
    time: Res<Time>,
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
    let sun_dir = if sun_direction.0.length_squared() > 0.0 {
        sun_direction.0.normalize()
    } else {
        Vec3::Z
    };
    let time_s = time.elapsed_secs();

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
                let lat_mask = aurora_lat_mask(lat, config.aurora_lat_start, config.aurora_lat_end);
                let mut value = aurora.grid_values[idx] * lat_mask;
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
                let night_mask = aurora_night_mask(&coords, sun_dir);
                if night_mask <= 0.0 {
                    continue;
                }
                let (u, v) = coords.convert_to_uv_mercator();
                let noise = sample_noise(
                    &render_state.noise_map,
                    render_state.noise_width,
                    render_state.noise_height,
                    u,
                    v,
                    time_s,
                    config.aurora_noise_speed,
                );
                let noise_factor = lerp(
                    1.0 - config.aurora_noise_strength,
                    1.0 + config.aurora_noise_strength,
                    noise,
                );
                value *= night_mask * noise_factor;
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
            let lat_mask =
                aurora_lat_mask(point.lat, config.aurora_lat_start, config.aurora_lat_end);
            let night_mask = aurora_night_mask(&coords, sun_dir);
            if lat_mask <= 0.0 || night_mask <= 0.0 {
                continue;
            }
            let noise = sample_noise(
                &render_state.noise_map,
                render_state.noise_width,
                render_state.noise_height,
                u,
                v,
                time_s,
                config.aurora_noise_speed,
            );
            let noise_factor = lerp(
                1.0 - config.aurora_noise_strength,
                1.0 + config.aurora_noise_strength,
                noise,
            );
            let x = (u * (render_state.width as f32 - 1.0))
                .round()
                .clamp(0.0, render_state.width as f32 - 1.0) as usize;
            let y = (v * (render_state.height as f32 - 1.0))
                .round()
                .clamp(0.0, render_state.height as f32 - 1.0) as usize;
            let idx = y * width + x;
            let value = point.value * lat_mask * night_mask * noise_factor;
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

    let floor = percentile_cutoff(&render_state.intensity_buffer, 0.8, max_value);

    for (i, chunk) in data.chunks_exact_mut(4).enumerate() {
        let raw = render_state.intensity_buffer[i];
        if raw <= floor || max_value <= floor {
            chunk[0] = 0;
            chunk[1] = 0;
            chunk[2] = 0;
            chunk[3] = 0;
            continue;
        }
        let normalized = ((raw - floor) / (max_value - floor)).clamp(0.0, 1.0);
        let scaled = (normalized * config.aurora_intensity_scale).clamp(0.0, 1.0);
        let shaped = scaled.powf(1.6);
        let intensity = (shaped * config.aurora_alpha).clamp(0.0, 1.0);
        if intensity <= 0.0 {
            chunk[0] = 0;
            chunk[1] = 0;
            chunk[2] = 0;
            chunk[3] = 0;
            continue;
        }
        let color = aurora_color(shaped);
        chunk[0] = (color[0] * intensity * 255.0).clamp(0.0, 255.0) as u8;
        chunk[1] = (color[1] * intensity * 255.0).clamp(0.0, 255.0) as u8;
        chunk[2] = (color[2] * intensity * 255.0).clamp(0.0, 255.0) as u8;
        chunk[3] = (intensity * 255.0).clamp(0.0, 255.0) as u8;
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

        let should_show =
            config.aurora_enabled && has_data && config.aurora_alpha > 0.0 && is_valid;
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
    // Dim green → bright green → red-orange
    let (start, mid, end) = ([0.05, 0.35, 0.12], [0.2, 1.0, 0.35], [1.0, 0.2, 0.2]);
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

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn aurora_lat_mask(lat: f32, start: f32, end: f32) -> f32 {
    let abs_lat = lat.abs();
    if abs_lat <= start {
        0.0
    } else if abs_lat >= end {
        1.0
    } else {
        let t = ((abs_lat - start) / (end - start)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
}

fn aurora_night_mask(coords: &Coordinates, sun_dir: Vec3) -> f32 {
    let normal_ecef = coords.get_point_on_sphere_ecef_km_dvec();
    let normal_bevy = ecef_to_bevy_km(normal_ecef).normalize_or_zero();
    let dot = normal_bevy.dot(sun_dir);
    smoothstep(0.1, -0.1, dot)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn generate_noise_map(width: usize, height: usize) -> Vec<f32> {
    let mut values = vec![0.0_f32; width * height];
    let mut state = 0x1234_abcd_u32;
    for y in 0..height {
        for x in 0..width {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let v = (state >> 8) as f32 / 16_777_215.0;
            values[y * width + x] = v;
        }
    }
    values
}

fn sample_noise(
    noise_map: &[f32],
    width: usize,
    height: usize,
    u: f32,
    v: f32,
    time_s: f32,
    speed: f32,
) -> f32 {
    if noise_map.is_empty() || width == 0 || height == 0 {
        return 0.5;
    }
    let u = (u + time_s * speed).fract();
    let v = (v + time_s * speed * 0.6).fract();
    let x = u * (width as f32 - 1.0);
    let y = v * (height as f32 - 1.0);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1) % width;
    let y1 = (y0 + 1) % height;
    let tx = x - x.floor();
    let ty = y - y.floor();
    let v00 = noise_map[y0 * width + x0];
    let v10 = noise_map[y0 * width + x1];
    let v01 = noise_map[y1 * width + x0];
    let v11 = noise_map[y1 * width + x1];
    let a = lerp(v00, v10, tx);
    let b = lerp(v01, v11, tx);
    lerp(a, b, ty)
}

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
