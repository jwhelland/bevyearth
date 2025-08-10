//! UI systems for the egui interface

use bevy::prelude::*;
use bevy::render::camera::Viewport;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContext, EguiContexts};
use bevy_egui::egui::Color32;
use chrono::SecondsFormat;

use crate::ui::state::{UIState, RightPanelUI};
use crate::satellite::{Satellite, SatelliteColor, SatelliteStore, SatEntry};
use crate::orbital::SimulationTime;
use crate::tle::{FetchChannels, FetchCommand};
use crate::visualization::ArrowConfig;
use crate::coverage::{CoverageParameters, FootprintConfig};
use crate::footprint_gizmo::{FootprintGizmo, FootprintGizmoConfig};
use crate::earth::EARTH_RADIUS_KM;

/// Main UI system that renders all the egui panels
pub fn ui_example_system(
    mut contexts: EguiContexts,
    mut camera: Single<&mut Camera, Without<EguiContext>>,
    window: Single<&mut Window, With<PrimaryWindow>>,
    mut state: ResMut<UIState>,
    mut arrows_cfg: ResMut<ArrowConfig>,
    mut footprint_cfg: ResMut<FootprintConfig>,
    mut gizmo_cfg: ResMut<FootprintGizmoConfig>,
    mut sim_time: ResMut<SimulationTime>,
    mut store: ResMut<SatelliteStore>,
    mut right_ui: ResMut<RightPanelUI>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    fetch_channels: Option<Res<FetchChannels>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
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

            ui.separator();
            ui.heading("Coverage Footprints");
            ui.separator();
            
            ui.checkbox(&mut footprint_cfg.enabled, "Show footprints");
            
            ui.collapsing("Default Parameters", |ui| {
                ui.add(egui::Slider::new(&mut footprint_cfg.default_frequency_mhz, 100.0..=30000.0)
                    .text("Frequency (MHz)"));
                ui.add(egui::Slider::new(&mut footprint_cfg.default_tx_power_dbm, 0.0..=50.0)
                    .text("TX Power (dBm)"));
                ui.add(egui::Slider::new(&mut footprint_cfg.default_antenna_gain_dbi, 0.0..=30.0)
                    .text("Antenna Gain (dBi)"));
                ui.add(egui::Slider::new(&mut footprint_cfg.default_min_signal_dbm, -150.0..=-50.0)
                    .text("Min Signal (dBm)"));
                ui.add(egui::Slider::new(&mut footprint_cfg.default_min_elevation_deg, 0.0..=45.0)
                    .text("Min Elevation (°)"));
            });
            
            ui.collapsing("Gizmo Settings", |ui| {
                ui.checkbox(&mut gizmo_cfg.enabled, "Use gizmo circles (recommended)");
                ui.add(egui::Slider::new(&mut gizmo_cfg.circle_segments, 16..=128)
                    .text("Circle segments"));
                ui.checkbox(&mut gizmo_cfg.show_signal_zones, "Show signal strength zones");
                ui.checkbox(&mut gizmo_cfg.show_center_dot, "Show center dot");
                if gizmo_cfg.show_center_dot {
                    ui.add(egui::Slider::new(&mut gizmo_cfg.center_dot_size, 50.0..=500.0)
                        .text("Center dot size (km)"));
                }
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
                            if !store.items.contains_key(&norad) {
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
                                store.items.insert(norad, SatEntry {
                                    norad,
                                    name: None,
                                    color,
                                    entity: Some(entity),
                                    tle: None,
                                    propagator: None,
                                    error: None,
                                    coverage_params: None,
                                    show_footprint: false,
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
            let mut to_remove: Option<u32> = None;
            let norad_keys: Vec<u32> = store.items.keys().copied().collect();
            for norad in norad_keys {
                // Use immutable access for display
                if let Some(s) = store.items.get(&norad) {
                    let mut remove = false;
                    let mut show_footprint = s.show_footprint;
                    ui.horizontal(|ui| {
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
                
                        // Add footprint checkbox if satellite is ready
                        if s.propagator.is_some() {
                            ui.checkbox(&mut show_footprint, "Coverage");
                        }
                
                        if ui.button("Remove").clicked() {
                            remove = true;
                        }
                    });
                    // Update show_footprint if changed
                    if s.propagator.is_some() {
                        if show_footprint != s.show_footprint {
                            if let Some(s_mut) = store.items.get_mut(&norad) {
                                s_mut.show_footprint = show_footprint;
                            }
                        }
                    }
                    if remove {
                        if let Some(s_mut) = store.items.get_mut(&norad) {
                            if let Some(entity) = s_mut.entity.take() {
                                // Bevy 0.16: despawn() recursively by default
                                commands.entity(entity).despawn();
                            }
                        }
                        to_remove = Some(norad);
                        break;
                    }
                }
            }
            if let Some(norad) = to_remove {
                store.items.remove(&norad);
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

    // System completed successfully
}