// Inspired by https://blog.graysonhead.net/posts/bevy-proc-earth-1/

use bevy::picking::prelude::*;
use bevy::render::mesh::Mesh;
use bevy::render::view::RenderLayers;
use bevy::{prelude::*, render::camera::Viewport, window::PrimaryWindow};

use bevy_egui::{
    egui, EguiContext, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass,
    PrimaryEguiContext,
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
// Additional imports
use bevy_egui::egui::Color32;
use bevy::math::DVec3;
use std::sync::{Arc, Mutex};

mod cities;
mod coord;
mod earth;
use crate::earth::EARTH_RADIUS_KM;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use cities::{spawn_city_population_spheres, CitiesEcef};
use coord::{hemisphere_prefilter, los_visible_ecef};
use earth::generate_faces;

// UI/state for dynamic satellites
#[derive(Component)]
struct Satellite;

#[derive(Component)]
struct SatelliteColor(pub Color);

#[derive(Resource, Default)]
struct SatelliteStore {
    items: Vec<SatEntry>,
    next_color_hue: f32,
}

struct SatEntry {
    norad: u32,
    name: Option<String>,
    color: Color,
    entity: Option<Entity>,
    // fetched TLE
    tle: Option<TleData>,
    // sgp4 2.3.0: hold parsed Constants and propagate per frame
    propagator: Option<sgp4::Constants>,
    // last error (if any)
    error: Option<String>,
}

#[derive(Resource, Default)]
struct RightPanelUI {
    input: String,
    error: Option<String>,
}

// Fetch plumbing
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
// For thread-safe receiver inside a Bevy Resource
// Arc<Mutex<...>> import is already present above

enum FetchCommand {
    Fetch(u32),
}

enum FetchResultMsg {
    Success {
        norad: u32,
        name: Option<String>,
        line1: String,
        line2: String,
        epoch_utc: DateTime<Utc>,
    },
    Failure { norad: u32, error: String },
}

#[derive(Resource)]
struct FetchChannels {
    cmd_tx: Sender<FetchCommand>,
    res_rx: Arc<Mutex<Receiver<FetchResultMsg>>>,
}

// TLE and utilities
struct TleData {
    name: Option<String>,
    line1: String,
    line2: String,
    epoch_utc: DateTime<Utc>,
}

fn parse_tle_epoch_to_utc(line1: &str) -> Option<DateTime<Utc>> {
    // TLE line1 epoch fields (columns 19–32, 1-based; 18..32 0-based)
    if line1.len() < 32 {
        return None;
    }
    let s = &line1[18..32];
    let mut parts = s.trim().split('.');
    let yyddd = parts.next()?;
    let frac = parts.next().unwrap_or("0");
    if yyddd.len() < 3 {
        return None;
    }
    let (yy_str, ddd_str) = yyddd.split_at(2);
    let yy: i32 = yy_str.parse().ok()?;
    let ddd: i32 = ddd_str.parse().ok()?;
    let year = if yy >= 57 { 1900 + yy } else { 2000 + yy };
    let jan1 = chrono::NaiveDate::from_ymd_opt(year, 1, 1)?;
    let date = jan1.checked_add_signed(chrono::Duration::days((ddd - 1) as i64))?;
    let frac_sec: f64 = match format!("0.{}", frac).parse::<f64>() {
        Ok(v) => v * 86400.0,
        Err(_) => return None,
    };
    let secs = frac_sec.trunc() as i64;
    let nanos = ((frac_sec - (secs as f64)) * 1e9).round() as i64;
    let dt = date.and_hms_opt(0, 0, 0)?;
    let mut ndt = chrono::NaiveDateTime::new(date, dt.time());
    ndt = ndt + chrono::Duration::seconds(secs);
    ndt = ndt + chrono::Duration::nanoseconds(nanos);
    Some(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
}

fn minutes_since_epoch(sim_utc: DateTime<Utc>, epoch: DateTime<Utc>) -> f64 {
    let delta = sim_utc - epoch;
    delta.num_seconds() as f64 / 60.0 + (delta.subsec_nanos() as f64) / 60.0 / 1.0e9
}

// Approximate GMST for visualization
fn gmst_rad(t: DateTime<Utc>) -> f64 {
    let secs = t.timestamp() as f64 + (t.timestamp_subsec_nanos() as f64) * 1e-9;
    let omega = std::f64::consts::TAU / 86164.0905_f64;
    (secs * omega).rem_euclid(std::f64::consts::TAU)
}

// Rotate ECI (TEME) -> ECEF using simple GMST rotation about Z
fn eci_to_ecef_km(eci: DVec3, gmst: f64) -> DVec3 {
    let (s, c) = gmst.sin_cos();
    let x = c * eci.x - s * eci.y;
    let y = s * eci.x + c * eci.y;
    DVec3::new(x, y, eci.z)
}

// Arrow rendering config (unchanged core)
#[derive(Resource)]
struct ArrowConfig {
    enabled: bool,
    color: Color,
    max_visible: usize,
    lift_m: f32,
    head_len_pct: f32,
    head_min_m: f32,
    head_max_m: f32,
    head_radius_pct: f32,
    shaft_len_pct: f32,
    shaft_min_m: f32,
    shaft_max_m: f32,
    gradient_enabled: bool,
    gradient_near_km: f32,
    gradient_far_km: f32,
    gradient_near_color: Color,
    gradient_far_color: Color,
    gradient_log_scale: bool,
}
impl Default for ArrowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            color: Color::srgb(0.1, 0.9, 0.3),
            max_visible: 200,
            lift_m: 10000.0,
            head_len_pct: 0.02,
            head_min_m: 10_000.0,
            head_max_m: 100_000.0,
            head_radius_pct: 0.4,
            shaft_len_pct: 0.05,
            shaft_min_m: 1_000.0,
            shaft_max_m: 400_000.0,
            gradient_enabled: false,
            gradient_near_km: 1000.0,
            gradient_far_km: 60000.0,
            gradient_near_color: Color::srgb(1.0, 0.0, 0.0),
            gradient_far_color: Color::srgb(0.0, 0.0, 1.0),
            gradient_log_scale: false,
        }
    }
}

// Satellite ECEF position resource (in kilometers)
#[derive(Resource, Deref, DerefMut, Default)]
struct SatEcef(pub Vec3);

#[derive(Resource)]
struct UIState {
    show_axes: bool,
}
impl Default for UIState {
    fn default() -> Self {
        Self { show_axes: false }
    }
}

/// Simulation time resource
#[derive(Resource)]
struct SimulationTime {
    current_utc: DateTime<Utc>,
    time_scale: f32,
}
impl Default for SimulationTime {
    fn default() -> Self {
        Self {
            current_utc: Utc::now(),
            time_scale: 1.0,
        }
    }
}

// Background TLE worker setup at startup
fn start_tle_worker() -> FetchChannels {
    let (cmd_tx, cmd_rx) = mpsc::channel::<FetchCommand>();
    let (res_tx, res_rx) = mpsc::channel::<FetchResultMsg>();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let client = reqwest::Client::new();

            // Helper: scan arbitrary response for a valid TLE pair, optionally with name
            fn extract_tle_block(body: &str, requested_sat: u32) -> anyhow::Result<(Option<String>, String, String)> {
                let mut lines: Vec<String> = Vec::new();
                for raw in body.lines() {
                    let line = raw.trim_matches(|c| c == '\u{feff}' || c == '\r' || c == '\n' || c == ' '); // trim BOM/CRLF/space
                    if line.is_empty() {
                        continue;
                    }
                    lines.push(line.to_string());
                }
                // find first pair 1/2 with matching sat number
                let sat_fmt = format!("{:05}", requested_sat);
                let mut i = 0usize;
                while i + 1 < lines.len() {
                    let l = &lines[i];
                    let n = if i >= 1 { Some(lines[i - 1].clone()) } else { None };
                    if l.starts_with('1') {
                        let l1 = l;
                        let l2 = &lines[i + 1];
                        if l2.starts_with('2') {
                            let sat_ok = l1.len() >= 7 && l2.len() >= 7 && &l1[2..7] == sat_fmt && &l2[2..7] == sat_fmt;
                            if sat_ok {
                                // Prefer a text name line immediately before l1 if it is not a TLE line
                                let name = if let Some(p) = n {
                                    if !p.starts_with('1') && !p.starts_with('2') { Some(p) } else { None }
                                } else { None };
                                return Ok((name, l1.to_string(), l2.to_string()));
                            }
                        }
                    }
                    i += 1;
                }
                let sample: String = body.lines().take(6).collect::<Vec<_>>().join("\\n");
                anyhow::bail!("No valid TLE pair found for {}. Sample: {}", requested_sat, sample);
            }

            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    FetchCommand::Fetch(norad) => {
                        let url = format!(
                            "https://celestrak.org/NORAD/elements/gp.php?CATNR={}&FORMAT=TLE",
                            norad
                        );
                        let send = |m| {
                            let _ = res_tx.send(m);
                        };
                        let res = async {
                            let resp = client
                                .get(&url)
                                .header("accept", "text/plain")
                                .send()
                                .await?;
                            let status = resp.status();
                            let body = resp.text().await?;
                            // Debug log full fetch result (status, first lines, and any extracted tuple)
                            println!("[TLE FETCH] norad={} status={} url={} bytes={}...", norad, status, url, body.len());
                            // Attempt parse even if not 2xx, to capture HTML/text bodies for debugging
                            let (name, l1, l2) = extract_tle_block(&body, norad)?;
                            println!("[TLE PARSED] norad={} name={}\\n{}\\n{}", norad, name.clone().unwrap_or_else(|| "None".into()), l1, l2);
                            // If HTTP not success, still bail after logging to surface error to UI
                            if !status.is_success() {
                                anyhow::bail!("HTTP {} after parse", status);
                            }
                            let epoch = parse_tle_epoch_to_utc(&l1).unwrap_or_else(|| Utc::now());
                            Ok::<_, anyhow::Error>((name, l1, l2, epoch))
                        }
                        .await;
                        match res {
                            Ok((name, line1, line2, epoch_utc)) => {
                                println!("[TLE RESULT] norad={} SUCCESS epoch={}", norad, epoch_utc.to_rfc3339());
                                send(FetchResultMsg::Success { norad, name, line1, line2, epoch_utc })
                            }
                            Err(e) => {
                                eprintln!("[TLE RESULT] norad={} FAILURE: {}", norad, e);
                                send(FetchResultMsg::Failure { norad, error: e.to_string() })
                            }
                        }
                    }
                }
            }
        });
    });
    FetchChannels { cmd_tx, res_rx: Arc::new(Mutex::new(res_rx)) }
}

// The `ShowAxes` component is attached to an entity to get the `draw_axes` system to display axes.
#[derive(Component)]
struct ShowAxes;

// Systems
fn update_satellite_ecef(
    sat_query: Query<&Transform, With<Satellite>>,
    mut sat_res: ResMut<SatEcef>,
) {
    if let Some(t) = sat_query.iter().next() {
        sat_res.0 = t.translation;
    }
}

fn draw_arrow_segment(
    gizmos: &mut Gizmos,
    city: Vec3,
    sat_pos: Vec3,
    fallback_color: Color,
    config: &ArrowConfig,
) {
    // constants conversion meters->kilometers
    let lift_km = config.lift_m / 1000.0;
    let head_min_km = config.head_min_m / 1000.0;
    let head_max_km = config.head_max_m / 1000.0;
    // Direction and lifted city endpoint
    let dir = (sat_pos - city).normalize();
    let city_lifted = city.normalize() * (EARTH_RADIUS_KM + lift_km);
    let total_len = (sat_pos - city_lifted).length();

    // color gradient
    let draw_color = if config.gradient_enabled {
        let mut near = config.gradient_near_km.max(1e-3);
        let mut far = config.gradient_far_km.max(near + 1e-3);
        if near > far {
            core::mem::swap(&mut near, &mut far);
        }
        let t = if config.gradient_log_scale {
            let ln = |x: f32| x.max(1e-3).ln();
            ((ln(total_len) - ln(near)) / (ln(far) - ln(near))).clamp(0.0, 1.0)
        } else {
            ((total_len - near) / (far - near)).clamp(0.0, 1.0)
        };
        config.gradient_near_color.mix(&config.gradient_far_color, t)
    } else {
        fallback_color
    };

    let mut shaft_len = config.shaft_len_pct * total_len;
    let shaft_min_km = config.shaft_min_m / 1000.0;
    let shaft_max_km = config.shaft_max_m / 1000.0;
    shaft_len = shaft_len
        .clamp(shaft_min_km, shaft_max_km)
        .min(total_len * 0.9);

    let shaft_end = city_lifted + dir * shaft_len;
    gizmos.arrow(city_lifted, shaft_end, draw_color);

    let _ = (head_min_km, head_max_km); // reserved for potential arrowhead
}

fn draw_city_to_satellite_arrows(
    mut gizmos: Gizmos,
    sat_query: Query<(&Transform, Option<&SatelliteColor>), With<Satellite>>,
    cities: Option<Res<CitiesEcef>>,
    config: Res<ArrowConfig>,
) {
    if !config.enabled {
        return;
    }
    let Some(cities) = cities else { return };
    let mut sats: Vec<(Vec3, Color)> = Vec::new();
    for (t, color_comp) in sat_query.iter() {
        let color = color_comp.map(|c| c.0).unwrap_or(config.color);
        sats.push((t.translation, color));
    }
    if sats.is_empty() {
        return;
    }

    let mut drawn = 0usize;
    'outer: for &city in cities.iter() {
        for &(sat_pos, sat_color) in &sats {
            if !hemisphere_prefilter(city, sat_pos, EARTH_RADIUS_KM) {
                continue;
            }
            if !los_visible_ecef(city, sat_pos, EARTH_RADIUS_KM) {
                continue;
            }
            draw_arrow_segment(&mut gizmos, city, sat_pos, sat_color, &config);
            drawn += 1;
            if drawn >= config.max_visible {
                break 'outer;
            }
        }
    }
}

fn draw_axes(mut gizmos: Gizmos, query: Query<&Transform, With<ShowAxes>>, state: Res<UIState>) {
    if !state.show_axes {
        return;
    }
    for &transform in &query {
        gizmos.axes(transform, 8000.0);
    }
}

// Advance simulation UTC by scale
fn advance_simulation_clock(time: Res<Time>, mut sim_time: ResMut<SimulationTime>) {
    let scaled = (time.delta_secs() * sim_time.time_scale).max(0.0);
    let whole = scaled.trunc() as i64;
    let nanos = ((scaled - scaled.trunc()) * 1_000_000_000.0) as i64;
    if whole != 0 {
        sim_time.current_utc = sim_time.current_utc + Duration::seconds(whole);
    }
    if nanos != 0 {
        sim_time.current_utc = sim_time.current_utc + Duration::nanoseconds(nanos);
    }
}

// Propagate satellites via SGP4 and set transforms
fn propagate_satellites_system(
    store: Res<SatelliteStore>,
    sim_time: Res<SimulationTime>,
    mut q: Query<(&mut Transform, &mut SatelliteColor, Entity), With<Satellite>>,
) {
    let gmst = gmst_rad(sim_time.current_utc);
    for entry in store.items.iter() {
        if let (Some(tle), Some(constants)) = (&entry.tle, &entry.propagator) {
            let mins = minutes_since_epoch(sim_time.current_utc, tle.epoch_utc);
            // sgp4 2.3.0 expects MinutesSinceEpoch newtype and returns arrays
            if let Ok(state) = constants.propagate(sgp4::MinutesSinceEpoch(mins)) {
                let pos = state.position; // [f64; 3] in km (TEME)
                let eci = DVec3::new(pos[0], pos[1], pos[2]);
                let ecef = eci_to_ecef_km(eci, gmst);
                let bevy_pos = Vec3::new(ecef.y as f32, ecef.z as f32, ecef.x as f32);
                if let Some((mut t, mut c, _)) =
                    q.iter_mut().find(|(_, _, e)| Some(*e) == entry.entity)
                {
                    t.translation = bevy_pos;
                    c.0 = entry.color;
                }
            }
        }
    }
}

// Setup scene, cameras, and TLE worker
pub fn setup(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    egui_global_settings.auto_create_primary_context = false;

    // Start TLE worker
    let channels = start_tle_worker();
    println!("[INIT] TLE worker started");
    commands.insert_resource(channels);

    // Axes marker
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap())),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0., 0.))),
        ShowAxes,
    ));
    commands.spawn((PanOrbitCamera::default(), Transform::from_xyz(25000.0, 8.0, 4.0)));
    commands.spawn((
        Camera2d,
        PrimaryEguiContext,
        RenderLayers::none(),
        Camera { order: 1, ..default() },
        Transform::from_xyz(25000.0, 8.0, 4.0),
    ));
}

// UI
fn ui_example_system(
    mut contexts: EguiContexts,
    mut camera: Single<&mut Camera, Without<EguiContext>>,
    window: Single<&mut Window, With<PrimaryWindow>>,
    mut state: ResMut<UIState>,
    mut arrows_cfg: ResMut<ArrowConfig>,
    mut sim_time: ResMut<SimulationTime>,
    mut store: ResMut<SatelliteStore>,
    mut right_ui: ResMut<RightPanelUI>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    fetch_channels: Option<Res<FetchChannels>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut left = egui::SidePanel::left("left_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.separator();

            ui.heading("Rendering");
            ui.separator();
            ui.checkbox(&mut state.show_axes, "Show axes");

            ui.separator();
            ui.heading("Simulation time");
            ui.horizontal(|ui| {
                ui.label("Scale:");
                ui.add(egui::Slider::new(&mut sim_time.time_scale, 0.0..=1000.0).logarithmic(true));
                if ui.button("1x").clicked() {
                    sim_time.time_scale = 1.0;
                }
            });

            ui.separator();
            ui.heading("Arrow rendering");
            ui.separator();

            ui.checkbox(&mut arrows_cfg.enabled, "Show arrows");
            ui.checkbox(
                &mut arrows_cfg.gradient_enabled,
                "Distance color gradient (red→blue)",
            );
            ui.collapsing("Gradient settings", |ui| {
                ui.label("Distance range (km)");
                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut arrows_cfg.gradient_near_km, 10.0..=200000.0).text("Near km"));
                    ui.add(egui::Slider::new(&mut arrows_cfg.gradient_far_km, 10.0..=200000.0).text("Far km"));
                });
                ui.checkbox(&mut arrows_cfg.gradient_log_scale, "Log scale");
            });
        })
        .response
        .rect
        .width();

    let mut right = egui::SidePanel::right("right_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Satellites");
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("NORAD:");
                let edit = ui.text_edit_singleline(&mut right_ui.input);
                let add_btn = ui.button("Add").clicked();
                let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if add_btn || enter {
                    match right_ui.input.trim().parse::<u32>() {
                        Ok(norad) => {
                            right_ui.error = None;
                            if !store.items.iter().any(|s| s.norad == norad) {
                                // randomized bright color for satellite marker (deterministic by NORAD)
                                // simple LCG to spread hues without external RNG dependency
                                let seed = (norad as u32)
                                    .wrapping_mul(1664525)
                                    .wrapping_add(1013904223);
                                let hue = (seed as f32 / u32::MAX as f32).fract(); // [0,1)
                                let sat = (0.65 + ((norad % 7) as f32) * 0.035).clamp(0.6, 0.9);
                                let light = (0.55 + ((norad % 11) as f32) * 0.02).clamp(0.5, 0.8);
                                let color = Color::hsl(hue, sat, light);

                                // spawn entity placeholder
                                let mesh = Sphere::new(100.0).mesh().ico(4).unwrap();
                                let entity = commands
                                    .spawn((
                                        Mesh3d(meshes.add(mesh)),
                                        MeshMaterial3d(materials.add(StandardMaterial {
                                            base_color: color,
                                            emissive: color.to_linear(),
                                            ..Default::default()
                                        })),
                                        Satellite,
                                        SatelliteColor(color),
                                        Transform::from_xyz(EARTH_RADIUS_KM + 5000.0, 0.0, 0.0),
                                    ))
                                    .id();
                                store.items.push(SatEntry {
                                    norad,
                                    name: None,
                                    color,
                                    entity: Some(entity),
                                    tle: None,
                                    propagator: None,
                                    error: None,
                                });
                                // Immediately send fetch request to background worker via injected resource
                                if let Some(fetch) = &fetch_channels {
                                    println!("[REQUEST] sending fetch for norad={}", norad);
                                    if let Err(e) = fetch.cmd_tx.send(FetchCommand::Fetch(norad)) {
                                        eprintln!("[REQUEST] failed to send fetch for norad={}: {}", norad, e);
                                    }
                                } else {
                                    eprintln!("[REQUEST] FetchChannels not available; cannot fetch norad={}", norad);
                                }
                                // clear input
                                right_ui.input.clear();
                            }
                        }
                        Err(_) => right_ui.error = Some("Invalid NORAD ID".to_string()),
                    }
                }
            });
            if let Some(err) = &right_ui.error {
                ui.colored_label(Color32::RED, err);
            }
            ui.separator();
            // list with basic status
            for idx in 0..store.items.len() {
                let mut remove = false;
                ui.horizontal(|ui| {
                    let s = &store.items[idx];
                    let status = if let Some(err) = &s.error {
                        format!("Error: {}", err)
                    } else if s.propagator.is_some() {
                        "Ready".to_string()
                    } else if s.tle.is_some() {
                        "TLE".to_string()
                    } else {
                        "Fetching...".to_string()
                    };
                    ui.label(format!(
                        "#{:>6}  {:<20} [{}]",
                        s.norad,
                        s.name.as_deref().unwrap_or("Unnamed"),
                        status
                    ));
                    if ui.button("Remove").clicked() {
                        remove = true;
                    }
                });
                if remove {
                    if let Some(entity) = store.items[idx].entity.take() {
                        // Bevy 0.16: despawn() recursively by default
                        commands.entity(entity).despawn();
                    }
                    store.items.remove(idx);
                    break;
                }
            }
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .width();

    let mut top = egui::TopBottomPanel::top("top_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("UTC:");
                ui.monospace(sim_time.current_utc.to_rfc3339_opts(SecondsFormat::Secs, true));
                if (sim_time.time_scale - 1.0).abs() > 1e-6 {
                    ui.separator();
                    ui.label(format!("{:.2}x", sim_time.time_scale));
                }
                ui.add_space(10.0);
                ui.separator();
            });
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .height();

    let mut bottom = egui::TopBottomPanel::bottom("bottom_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Bottom resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .height();

    // Scale from logical units to physical units.
    left *= window.scale_factor();
    right *= window.scale_factor();
    top *= window.scale_factor();
    bottom *= window.scale_factor();

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

// Drain fetch results and build propagators
fn process_fetch_results_system(
    mut store: ResMut<SatelliteStore>,
    fetch: Option<Res<FetchChannels>>,
) {
    let Some(fetch) = fetch else { return };
    let Ok(guard) = fetch.res_rx.lock() else { return };
    while let Ok(msg) = guard.try_recv() {
        match msg {
            FetchResultMsg::Success {
                norad,
                name,
                line1,
                line2,
                epoch_utc,
            } => {
                println!("[TLE DISPATCH] received SUCCESS for norad={}", norad);
                if let Some(s) = store.items.iter_mut().find(|s| s.norad == norad) {
                    // clear previous error
                    s.error = None;
                    s.name = name.or_else(|| Some(format!("NORAD {}", norad)));
                    let epoch = parse_tle_epoch_to_utc(&line1).unwrap_or(epoch_utc);
                    s.tle = Some(TleData {
                        name: s.name.clone(),
                        line1: line1.clone(),
                        line2: line2.clone(),
                        epoch_utc: epoch,
                    });
                    // Build SGP4 model (sgp4 2.3.0): parse TLE -> Elements -> Constants
                    match sgp4::Elements::from_tle(s.name.clone(), line1.as_bytes(), line2.as_bytes()) {
                        Ok(elements) => match sgp4::Constants::from_elements(&elements) {
                            Ok(constants) => {
                                s.propagator = Some(constants);
                                println!("[SGP4] norad={} constants initialized", norad);
                            }
                            Err(e) => {
                                s.propagator = None;
                                s.error = Some(e.to_string());
                                eprintln!("[SGP4] norad={} constants error: {}", norad, s.error.as_deref().unwrap());
                            }
                        },
                        Err(e) => {
                            s.propagator = None;
                            s.error = Some(e.to_string());
                            eprintln!("[SGP4] norad={} elements error: {}", norad, s.error.as_deref().unwrap());
                        }
                    }
                } else {
                    eprintln!("[TLE DISPATCH] norad={} not found in store", norad);
                }
            }
            FetchResultMsg::Failure { norad, error } => {
                eprintln!("[TLE DISPATCH] received FAILURE for norad={}: {}", norad, error);
                if let Some(s) = store.items.iter_mut().find(|s| s.norad == norad) {
                    // keep existing name if any; record error and clear models
                    s.error = Some(error);
                    s.tle = None;
                    s.propagator = None;
                } else {
                    eprintln!("[TLE DISPATCH] failure for unknown norad={} (not in store)", norad);
                }
            }
        }
    }
}

fn main() {
    App::new()
        .init_resource::<UIState>()
        .init_resource::<ArrowConfig>()
        .init_resource::<SatEcef>()
        .init_resource::<SimulationTime>()
        .init_resource::<SatelliteStore>()
        .init_resource::<RightPanelUI>()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(MeshPickingPlugin)
        .add_systems(Startup, (setup, generate_faces, spawn_city_population_spheres).chain())
        .add_systems(
            Update,
            (
                draw_axes.after(setup),
                advance_simulation_clock,               // advance UTC
                process_fetch_results_system,           // receive TLEs/models
                propagate_satellites_system.after(advance_simulation_clock), // update sat transforms
                update_satellite_ecef.after(propagate_satellites_system),
                draw_city_to_satellite_arrows.after(propagate_satellites_system),
            ),
        )
        .add_systems(EguiPrimaryContextPass, ui_example_system)
        .run();
}
