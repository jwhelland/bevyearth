//! UI systems for the egui interface

use bevy::prelude::*;
use bevy::render::camera::Viewport;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContext, EguiContexts};
use bevy_egui::egui::Color32;
use chrono::SecondsFormat;

use crate::ui::state::{UIState, RightPanelUI};
use crate::ui::groups::{SATELLITE_GROUPS, get_group_display_name};
use crate::satellite::{Satellite, SatelliteColor, SatelliteStore, SatEntry, OrbitTrailConfig};
use crate::orbital::SimulationTime;
use crate::tle::{FetchChannels, FetchCommand};
use crate::visualization::ArrowConfig;
use crate::coverage::FootprintConfig;
use crate::footprint_gizmo::FootprintGizmoConfig;
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
    mut trail_cfg: ResMut<OrbitTrailConfig>,
    mut sim_time: ResMut<SimulationTime>,
    mut store: ResMut<SatelliteStore>,
    mut right_ui: ResMut<RightPanelUI>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    fetch_channels: Option<Res<FetchChannels>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    
    // Handle keyboard shortcuts for panel toggles
    ctx.input(|i| {
        if i.key_pressed(egui::Key::H) {
            state.show_left_panel = !state.show_left_panel;
        }
        if i.key_pressed(egui::Key::J) {
            state.show_right_panel = !state.show_right_panel;
        }
        if i.key_pressed(egui::Key::K) {
            state.show_top_panel = !state.show_top_panel;
        }
        if i.key_pressed(egui::Key::L) {
            state.show_bottom_panel = !state.show_bottom_panel;
        }
    });
    let mut left = 0.0;
    if state.show_left_panel {
        left = egui::SidePanel::left("left_panel")
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

            ui.separator();
            ui.heading("Orbit Trails");
            ui.separator();
            
            ui.add(egui::Slider::new(&mut trail_cfg.max_points, 10..=500)
                .text("History points"));
            ui.add(egui::Slider::new(&mut trail_cfg.max_age_seconds, 60.0..=1800.0)
                .text("Max age (seconds)"));
            ui.add(egui::Slider::new(&mut trail_cfg.update_interval_seconds, 0.5..=10.0)
                .text("Update interval (seconds)"));
        })
        .response
        .rect
        .width();
    }

    let mut right = 0.0;
    if state.show_right_panel {
        right = egui::SidePanel::right("right_panel")
            .resizable(true)
            .show(ctx, |ui| {
            ui.heading("Satellites");
            ui.separator();
            
            ui.collapsing("Satellite Groups", |ui| {
                ui.separator();
            
                // Group selection dropdown
                egui::ComboBox::from_label("Select Group")
                    .selected_text(
                        right_ui.selected_group
                            .as_ref()
                            .map(|g| get_group_display_name(g))
                            .unwrap_or("Choose a group...")
                    )
                    .show_ui(ui, |ui| {
                        for (group_key, group_name) in SATELLITE_GROUPS {
                            ui.selectable_value(&mut right_ui.selected_group, Some(group_key.to_string()), *group_name);
                        }
                    });
            
                let mut load_group_request = None;
                let mut set_error = None;
                let mut set_loading = None;
            
                ui.horizontal(|ui| {
                    let load_btn = ui.button("Load Group").clicked();
                    if right_ui.group_loading {
                        ui.spinner();
                        ui.label("Loading...");
                    }
                    
                    if load_btn && !right_ui.group_loading {
                        if let Some(group) = &right_ui.selected_group {
                            load_group_request = Some(group.clone());
                            set_loading = Some(true);
                            set_error = Some(None);
                        } else {
                            set_error = Some(Some("Please select a group first".to_string()));
                        }
                    }
                });
            
                // Handle the group loading request outside the closure
                if let Some(group) = load_group_request {
                    right_ui.group_loading = true;
                    right_ui.error = None;
                    
                    // Send group fetch command
                    if let Some(fetch) = &fetch_channels {
                        println!("[REQUEST] sending group fetch for group={}", group);
                        if let Err(e) = fetch.cmd_tx.send(FetchCommand::FetchGroup { group }) {
                            eprintln!("[REQUEST] failed to send group fetch: {}", e);
                            right_ui.error = Some(format!("Failed to request group: {}", e));
                            right_ui.group_loading = false;
                        }
                    } else {
                        eprintln!("[REQUEST] FetchChannels not available; cannot fetch group");
                        right_ui.error = Some("Fetch service not available".to_string());
                        right_ui.group_loading = false;
                    }
                }
            
                // Apply any error state changes
                if let Some(error) = set_error {
                    right_ui.error = error;
                }
            
                ui.separator();
            });
            
            ui.collapsing("Individual Satellites", |ui| {
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
                                    let seed = norad
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
                                                // base_color: color,
                                                emissive: color.to_linear() * 20.0,
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
                                        show_trail: false,
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
            });
            
            // Clear All button
            ui.horizontal(|ui| {
                if ui.button("Clear All Satellites").clicked() {
                    // Mark all satellites for removal
                    for (_, entry) in store.items.iter_mut() {
                        if let Some(entity) = entry.entity.take() {
                            commands.entity(entity).despawn();
                        }
                    }
                    store.items.clear();
                }
                ui.label(format!("({} satellites)", store.items.len()));
            });
            ui.separator();
            
            ui.collapsing("Master Controls", |ui| {
                ui.separator();
            
                // Compute current master states
                let ready_satellites: Vec<_> = store.items.values()
                    .filter(|s| s.propagator.is_some())
                    .collect();
            
                let all_coverage_enabled = !ready_satellites.is_empty() &&
                    ready_satellites.iter().all(|s| s.show_footprint);
                let all_trails_enabled = !ready_satellites.is_empty() &&
                    ready_satellites.iter().all(|s| s.show_trail);
            
                // Master coverage checkbox
                let mut master_coverage = all_coverage_enabled;
                if ui.checkbox(&mut master_coverage, "All Coverage").changed() {
                    for entry in store.items.values_mut() {
                        if entry.propagator.is_some() {
                            entry.show_footprint = master_coverage;
                        }
                    }
                }
            
                // Master trails checkbox
                let mut master_trails = all_trails_enabled;
                if ui.checkbox(&mut master_trails, "All Trails").changed() {
                    for entry in store.items.values_mut() {
                        if entry.propagator.is_some() {
                            entry.show_trail = master_trails;
                        }
                    }
                }
            
                ui.separator();
            });
            
            // Satellite table view
            let mut to_remove: Option<u32> = None;
            let norad_keys: Vec<u32> = store.items.keys().copied().collect();
            
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    use egui_extras::{TableBuilder, Column};
                    
                    TableBuilder::new(ui)
                        .column(Column::exact(50.0)) // NORAD ID
                        .column(Column::remainder().at_least(80.0)) // Name
                        .column(Column::exact(60.0)) // Status
                        .column(Column::exact(50.0)) // Coverage
                        .column(Column::exact(50.0)) // Trail
                        .column(Column::exact(50.0)) // Actions
                        .header(20.0, |mut header| {
                            header.col(|ui| {
                                ui.strong("NORAD");
                            });
                            header.col(|ui| {
                                ui.strong("Name");
                            });
                            header.col(|ui| {
                                ui.strong("Status");
                            });
                            header.col(|ui| {
                                ui.strong("Cov");
                            });
                            header.col(|ui| {
                                ui.strong("Trail");
                            });
                            header.col(|ui| {
                                ui.strong("Act");
                            });
                        })
                        .body(|mut body| {
                            for norad in norad_keys {
                                // Use immutable access for display, collect changes to apply later
                                if let Some(s) = store.items.get(&norad) {
                                    let mut remove = false;
                                    let mut show_footprint = s.show_footprint;
                                    let mut show_trail = s.show_trail;
                                    let has_propagator = s.propagator.is_some();
                                    let old_footprint = s.show_footprint;
                                    let old_trail = s.show_trail;
                                    
                                    body.row(18.0, |mut row| {
                                        // NORAD ID column
                                        row.col(|ui| {
                                            ui.label(format!("{}", s.norad));
                                        });
                                        
                                        // Name column
                                        row.col(|ui| {
                                            ui.add(egui::Label::new(s.name.as_deref().unwrap_or("Unnamed")).truncate());
                                        });
                                        
                                        // Status column with color coding
                                        row.col(|ui| {
                                            let (status_text, status_color) = if let Some(_err) = &s.error {
                                                ("Error", Color32::RED)
                                            } else if s.propagator.is_some() {
                                                ("Ready", Color32::GREEN)
                                            } else if s.tle.is_some() {
                                                ("TLE", Color32::YELLOW)
                                            } else {
                                                ("Fetching", Color32::GRAY)
                                            };
                                            ui.colored_label(status_color, status_text);
                                        });
                                        
                                        // Coverage checkbox column
                                        row.col(|ui| {
                                            if has_propagator {
                                                ui.checkbox(&mut show_footprint, "");
                                            } else {
                                                ui.add_enabled(false, egui::Checkbox::new(&mut false, ""));
                                            }
                                        });
                                        
                                        // Trail checkbox column
                                        row.col(|ui| {
                                            if has_propagator {
                                                ui.checkbox(&mut show_trail, "");
                                            } else {
                                                ui.add_enabled(false, egui::Checkbox::new(&mut false, ""));
                                            }
                                        });
                                        
                                        // Actions column
                                        row.col(|ui| {
                                            if ui.small_button("Remove").clicked() {
                                                remove = true;
                                            }
                                        });
                                    });
                                    
                                    // Apply changes after releasing immutable borrow
                                    let _ = s;
                                    
                                    // Update show_footprint if changed
                                    if has_propagator && show_footprint != old_footprint {
                                        if let Some(s_mut) = store.items.get_mut(&norad) {
                                            s_mut.show_footprint = show_footprint;
                                        }
                                    }
                                    // Update show_trail if changed
                                    if has_propagator && show_trail != old_trail {
                                        if let Some(s_mut) = store.items.get_mut(&norad) {
                                            s_mut.show_trail = show_trail;
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
                        });
                });
            
            if let Some(norad) = to_remove {
                store.items.remove(&norad);
            }
            
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .width();
    }

    let mut top = 0.0;
    if state.show_top_panel {
        top = egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Time display
                ui.strong("UTC:");
                ui.monospace(sim_time.current_utc.to_rfc3339_opts(SecondsFormat::Secs, true));
                if (sim_time.time_scale - 1.0).abs() > 1e-6 {
                    ui.separator();
                    ui.label(format!("{:.2}x", sim_time.time_scale));
                }
                ui.add_space(10.0);
                ui.separator();
                
                // Panel toggle buttons
                ui.label("Panels:");
                if ui.small_button(if state.show_left_panel { "Hide Left (H)" } else { "Show Left (H)" }).clicked() {
                    state.show_left_panel = !state.show_left_panel;
                }
                if ui.small_button(if state.show_right_panel { "Hide Right (J)" } else { "Show Right (J)" }).clicked() {
                    state.show_right_panel = !state.show_right_panel;
                }
                if ui.small_button(if state.show_top_panel { "Hide Top (K)" } else { "Show Top (K)" }).clicked() {
                    state.show_top_panel = !state.show_top_panel;
                }
                if ui.small_button(if state.show_bottom_panel { "Hide Bottom (L)" } else { "Show Bottom (L)" }).clicked() {
                    state.show_bottom_panel = !state.show_bottom_panel;
                }
            });
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .height();
    }

    let mut bottom = 0.0;
    if state.show_bottom_panel {
        bottom = egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Debug Info:");
                ui.separator();
                ui.label(format!("Satellites: {}", store.items.len()));
                if let Some(_fetch) = &fetch_channels {
                    ui.separator();
                    ui.label("TLE Fetcher: Active");
                } else {
                    ui.separator();
                    ui.colored_label(Color32::RED, "TLE Fetcher: Inactive");
                }
            });
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .height();
    }

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