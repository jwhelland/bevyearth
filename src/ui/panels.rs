//! UI panel components and utilities
use bevy::prelude::*;
use bevy_egui::egui::{self, Color32};
use chrono::SecondsFormat;

use crate::core::coordinates::EARTH_RADIUS_KM;
use crate::orbital::SimulationTime;
use crate::satellite::{SatEntry, Satellite, SatelliteColor, SatelliteStore, SelectedSatellite};
use crate::tle::{FetchChannels, FetchCommand};
use crate::ui::groups::{SATELLITE_GROUPS, get_group_display_name};
use crate::ui::state::{RightPanelUI, UIState};
use crate::visualization::{ArrowConfig, HeatmapConfig, RangeMode};

/// Convert Bevy Color to egui Color32
fn bevy_to_egui_color(color: Color) -> Color32 {
    let srgba = color.to_srgba();
    Color32::from_rgb(
        (srgba.red * 255.0) as u8,
        (srgba.green * 255.0) as u8,
        (srgba.blue * 255.0) as u8,
    )
}

pub fn render_left_panel(
    ui: &mut egui::Ui,
    arrows_cfg: &mut ArrowConfig,
    sim_time: &mut SimulationTime,
) {
    // ui.separator();

    // ui.heading("Rendering");
    // ui.separator();
    // ui.checkbox(&mut state.show_axes, "Show axes");

    ui.separator();
    ui.heading("Speedup time");
    ui.horizontal(|ui| {
        ui.label("Scale:");
        ui.add(egui::Slider::new(&mut sim_time.time_scale, 1.0..=1000.0).logarithmic(false));
        if ui.button("1x").clicked() {
            sim_time.time_scale = 1.0;
        }
        if ui.button("Now").clicked() {
            sim_time.current_utc = chrono::Utc::now();
            sim_time.time_scale = 1.0;
        }
    });

    ui.separator();
    ui.heading("City -> Sat Vis");
    ui.separator();

    ui.checkbox(&mut arrows_cfg.enabled, "Show arrows");
    ui.checkbox(
        &mut arrows_cfg.gradient_enabled,
        "Distance color gradient (red→blue)",
    );
    ui.collapsing("Gradient settings", |ui| {
        ui.label("Distance range (km)");
        ui.horizontal(|ui| {
            ui.add(
                egui::Slider::new(&mut arrows_cfg.gradient_near_km, 10.0..=200000.0)
                    .text("Near km"),
            );
            ui.add(
                egui::Slider::new(&mut arrows_cfg.gradient_far_km, 10.0..=200000.0).text("Far km"),
            );
        });
        ui.checkbox(&mut arrows_cfg.gradient_log_scale, "Log scale");
    });
    
}

pub fn render_right_panel(
    ui: &mut egui::Ui,
    store: &mut SatelliteStore,
    right_ui: &mut RightPanelUI,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    selected_sat: &mut SelectedSatellite,
    config_bundle: &mut crate::ui::systems::UiConfigBundle,
    heatmap_cfg: &mut HeatmapConfig,
    fetch_channels: &Option<Res<FetchChannels>>,
) {
    ui.heading("Satellites");
    ui.separator();

    egui::CollapsingHeader::new("Satellite Groups")
        .default_open(true)
        .show(ui, |ui| {
            ui.separator();

            // Group selection dropdown
            egui::ComboBox::from_label("Select Group")
                .selected_text(
                    right_ui
                        .selected_group
                        .as_ref()
                        .map(|g| get_group_display_name(g))
                        .unwrap_or("Choose a group..."),
                )
                .show_ui(ui, |ui| {
                    for (group_key, group_name) in SATELLITE_GROUPS {
                        ui.selectable_value(
                            &mut right_ui.selected_group,
                            Some(group_key.to_string()),
                            *group_name,
                        );
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
                if let Some(fetch) = fetch_channels {
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
                            let seed = norad.wrapping_mul(1664525).wrapping_add(1013904223);
                            let hue = (seed as f32 / u32::MAX as f32).fract(); // [0,1)
                            let sat = (0.65 + ((norad % 7) as f32) * 0.035).clamp(0.6, 0.9);
                            let light = (0.55 + ((norad % 11) as f32) * 0.02).clamp(0.5, 0.8);
                            let color = Color::hsl(hue, sat, light);

                            // spawn entity placeholder
                            let mesh = Sphere::new(1.0).mesh().ico(4).unwrap();
                            let entity = commands
                                .spawn((
                                    Mesh3d(meshes.add(mesh)),
                                    MeshMaterial3d(materials.add(StandardMaterial {
                                        // base_color: color,
                                        emissive: color.to_linear()
                                            * config_bundle.render_cfg.emissive_intensity,
                                        ..Default::default()
                                    })),
                                    Satellite,
                                    SatelliteColor(color),
                                    Transform::from_xyz(EARTH_RADIUS_KM + 5000.0, 0.0, 0.0)
                                        .with_scale(Vec3::splat(
                                            config_bundle.render_cfg.sphere_radius,
                                        )),
                                ))
                                .id();
                            store.items.insert(
                                norad,
                                SatEntry {
                                    norad,
                                    name: None,
                                    color,
                                    entity: Some(entity),
                                    tle: None,
                                    propagator: None,
                                    error: None,
                                    show_ground_track: false,
                                    show_trail: false,
                                    is_clicked: false,
                                },
                            );
                            // Immediately send fetch request to background worker via injected resource
                            if let Some(fetch) = fetch_channels {
                                if let Err(e) = fetch.cmd_tx.send(FetchCommand::Fetch(norad)) {
                                    eprintln!(
                                        "[REQUEST] failed to send fetch for norad={}: {}",
                                        norad, e
                                    );
                                }
                            } else {
                                eprintln!(
                                    "[REQUEST] FetchChannels not available; cannot fetch norad={}",
                                    norad
                                );
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

    ui.collapsing("Ground Track Settings", |ui| {
        ui.separator();

        // Compute current master states for ground tracks
        let ready_satellites: Vec<_> = store
            .items
            .values()
            .filter(|s| s.propagator.is_some())
            .collect();

        let all_ground_tracks_enabled =
            !ready_satellites.is_empty() && ready_satellites.iter().all(|s| s.show_ground_track);

        // Master ground track checkbox
        let mut master_ground_track = all_ground_tracks_enabled;
        if ui
            .checkbox(&mut master_ground_track, "All Tracks")
            .changed()
        {
            for entry in store.items.values_mut() {
                if entry.propagator.is_some() {
                    entry.show_ground_track = master_ground_track;
                }
            }
        }

        ui.separator();

        ui.checkbox(
            &mut config_bundle.ground_track_cfg.enabled,
            "Show ground tracks",
        );
        ui.add(
            egui::Slider::new(&mut config_bundle.ground_track_cfg.radius_km, 10.0..=500.0)
                .text("Track radius (km)"),
        );

        ui.collapsing("Gizmo Settings", |ui| {
            ui.checkbox(
                &mut config_bundle.gizmo_cfg.enabled,
                "Use gizmo circles (recommended)",
            );
            ui.add(
                egui::Slider::new(&mut config_bundle.gizmo_cfg.circle_segments, 16..=128)
                    .text("Circle segments"),
            );
            ui.checkbox(
                &mut config_bundle.gizmo_cfg.show_center_dot,
                "Show center dot",
            );
            if config_bundle.gizmo_cfg.show_center_dot {
                ui.add(
                    egui::Slider::new(&mut config_bundle.gizmo_cfg.center_dot_size, 50.0..=500.0)
                        .text("Center dot size (km)"),
                );
            }
        });

        ui.separator();
    });

    ui.collapsing("Orbit Trail Settings", |ui| {
        ui.separator();

        // Compute current master states for trails
        let ready_satellites: Vec<_> = store
            .items
            .values()
            .filter(|s| s.propagator.is_some())
            .collect();

        let all_trails_enabled =
            !ready_satellites.is_empty() && ready_satellites.iter().all(|s| s.show_trail);

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

        ui.add(
            egui::Slider::new(&mut config_bundle.trail_cfg.max_points, 100..=10000)
                .text("Max history points"),
        );
        ui.add(
            egui::Slider::new(
                &mut config_bundle.trail_cfg.update_interval_seconds,
                0.5..=10.0,
            )
            .text("Update interval (seconds)"),
        );

        ui.separator();
    });

    ui.collapsing("Heatmap Settings", |ui| {
        ui.separator();

        if ui.checkbox(&mut heatmap_cfg.enabled, "Enable heatmap").changed() {
            info!("Heatmap checkbox clicked! New value: {}", heatmap_cfg.enabled);
        }

        if heatmap_cfg.enabled {
            ui.horizontal(|ui| {
                ui.label("Update period:");
                ui.add(egui::Slider::new(&mut heatmap_cfg.update_period_s, 0.1..=2.0).text("seconds"));
            });

            ui.horizontal(|ui| {
                ui.label("Opacity:");
                ui.add(egui::Slider::new(&mut heatmap_cfg.color_alpha, 0.0..=1.0).text("alpha"));
            });

            ui.horizontal(|ui| {
                ui.label("Range mode:");
                ui.radio_value(&mut heatmap_cfg.range_mode, RangeMode::Auto, "Auto");
                ui.radio_value(&mut heatmap_cfg.range_mode, RangeMode::Fixed, "Fixed");
            });

            if heatmap_cfg.range_mode == RangeMode::Fixed {
                ui.horizontal(|ui| {
                    ui.label("Fixed max:");
                    if let Some(ref mut fixed_max) = heatmap_cfg.fixed_max {
                        ui.add(egui::Slider::new(fixed_max, 1..=100).text("satellites"));
                    } else {
                        heatmap_cfg.fixed_max = Some(20);
                    }
                });
            }

            ui.collapsing("Performance Tuning", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Chunk size:");
                    ui.add(egui::Slider::new(&mut heatmap_cfg.chunk_size, 500..=5000).text("vertices"));
                });
                ui.horizontal(|ui| {
                    ui.label("Chunks/frame:");
                    ui.add(egui::Slider::new(&mut heatmap_cfg.chunks_per_frame, 1..=5).text("chunks"));
                });
            });
        }

        ui.separator();
    });

    ui.collapsing("Satellite Rendering", |ui| {
        ui.separator();

        ui.add(
            egui::Slider::new(&mut config_bundle.render_cfg.sphere_radius, 1.0..=200.0)
                .text("Sphere size (km)"),
        );
        ui.add(
            egui::Slider::new(
                &mut config_bundle.render_cfg.emissive_intensity,
                10.0..=500.0,
            )
            .text("Emissive intensity"),
        );

        ui.separator();
    });

    // Tracking Controls Section
    ui.collapsing("Camera Tracking", |ui| {
        ui.separator();

        // Show current tracking status
        if let Some(tracking_norad) = selected_sat.tracking {
            if let Some(entry) = store.items.get(&tracking_norad) {
                let sat_name = entry.name.as_deref().unwrap_or("Unnamed");
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::GREEN, "📹 Tracking:");
                    ui.colored_label(
                        bevy_to_egui_color(entry.color),
                        format!("{} ({})", sat_name, tracking_norad),
                    );
                });

                // Stop Tracking button
                if ui.button("Stop Tracking").clicked() {
                    selected_sat.tracking = None;
                }

                ui.separator();

                // Tracking configuration
                ui.label("Tracking Settings:");
                ui.add(
                    egui::Slider::new(&mut selected_sat.tracking_offset, 1000.0..=20000.0)
                        .text("Distance (km)"),
                );
                ui.add(
                    egui::Slider::new(&mut selected_sat.smooth_factor, 0.01..=1.0)
                        .text("Smoothness"),
                );
            }
        } else {
            ui.colored_label(Color32::GRAY, "📹 Not tracking any satellite");
            ui.label("Click a satellite NORAD ID to start tracking");
        }

        ui.separator();
    });

    ui.separator();

    // Satellite table view
    let mut to_remove: Option<u32> = None;
    let norad_keys: Vec<u32> = store.items.keys().copied().collect();

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            use egui_extras::{Column, TableBuilder};

            TableBuilder::new(ui)
                .column(Column::exact(50.0)) // NORAD ID
                .column(Column::remainder().at_least(80.0)) // Name
                .column(Column::exact(60.0)) // Status
                .column(Column::exact(50.0)) // Ground Track
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
                        ui.strong("Track");
                    });
                    header.col(|ui| {
                        ui.strong("Trail");
                    });
                    header.col(|ui| {
                        ui.strong("");
                    });
                })
                .body(|mut body| {
                    for norad in norad_keys {
                        // Use immutable access for display, collect changes to apply later
                        if let Some(s) = store.items.get(&norad) {
                            let mut remove = false;
                            let mut show_ground_track = s.show_ground_track;
                            let mut show_trail = s.show_trail;
                            let has_propagator = s.propagator.is_some();
                            let old_ground_track = s.show_ground_track;
                            let old_trail = s.show_trail;

                            body.row(18.0, |mut row| {
                                // NORAD ID column (clickable)
                                row.col(|ui| {
                                    let is_tracking = selected_sat.tracking == Some(s.norad);
                                    let button_text = if is_tracking {
                                        format!("📹 {}", s.norad)
                                    } else {
                                        format!("{}", s.norad)
                                    };

                                    let mut button = egui::Button::new(
                                        egui::RichText::new(button_text)
                                            .color(bevy_to_egui_color(s.color)),
                                    );

                                    // Highlight tracking button
                                    if is_tracking {
                                        button = button.fill(Color32::from_rgb(0, 50, 0));
                                    }

                                    if ui.add(button).clicked() {
                                        if selected_sat.tracking == Some(s.norad) {
                                            // Currently tracking this satellite, so untrack it
                                            selected_sat.tracking = None;
                                        } else {
                                            // Not tracking this satellite, so start tracking it
                                            selected_sat.selected = Some(s.norad);
                                            selected_sat.tracking = Some(s.norad);
                                        }
                                    }
                                });

                                // Name column
                                row.col(|ui| {
                                    ui.add(
                                        egui::Label::new(s.name.as_deref().unwrap_or("Unnamed"))
                                            .truncate(),
                                    );
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

                                // Ground Track checkbox column
                                row.col(|ui| {
                                    if has_propagator {
                                        ui.checkbox(&mut show_ground_track, "");
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
                                    if ui.small_button("x").clicked() {
                                        remove = true;
                                    }
                                });
                            });

                            // Apply changes after releasing immutable borrow
                            let s_norad = s.norad;

                            // Update show_ground_track if changed
                            if has_propagator && show_ground_track != old_ground_track {
                                if let Some(s_mut) = store.items.get_mut(&s_norad) {
                                    s_mut.show_ground_track = show_ground_track;
                                }
                            }
                            // Update show_trail if changed
                            if has_propagator && show_trail != old_trail {
                                if let Some(s_mut) = store.items.get_mut(&s_norad) {
                                    s_mut.show_trail = show_trail;
                                }
                            }
                            if remove {
                                if let Some(s_mut) = store.items.get_mut(&s_norad) {
                                    if let Some(entity) = s_mut.entity.take() {
                                        // Bevy 0.16: despawn() recursively by default
                                        commands.entity(entity).despawn();
                                    }
                                }
                                to_remove = Some(s_norad);
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
}

pub fn render_top_panel(ui: &mut egui::Ui, state: &mut UIState, sim_time: &SimulationTime) {
    ui.horizontal(|ui| {
        // Time display
        ui.strong("UTC:");
        ui.monospace(
            sim_time
                .current_utc
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        );
        if (sim_time.time_scale - 1.0).abs() > 1e-6 {
            ui.separator();
            ui.label(format!("{:.2}x", sim_time.time_scale));
        }
        ui.add_space(10.0);
        ui.separator();

        // Panel toggle buttons
        ui.label("Panels:");
        if ui
            .small_button(if state.show_left_panel {
                "Hide Left (H)"
            } else {
                "Show Left (H)"
            })
            .clicked()
        {
            state.show_left_panel = !state.show_left_panel;
        }
        if ui
            .small_button(if state.show_right_panel {
                "Hide Right (J)"
            } else {
                "Show Right (J)"
            })
            .clicked()
        {
            state.show_right_panel = !state.show_right_panel;
        }
        if ui
            .small_button(if state.show_top_panel {
                "Hide Top (K)"
            } else {
                "Show Top (K)"
            })
            .clicked()
        {
            state.show_top_panel = !state.show_top_panel;
        }
        if ui
            .small_button(if state.show_bottom_panel {
                "Hide Bottom (L)"
            } else {
                "Show Bottom (L)"
            })
            .clicked()
        {
            state.show_bottom_panel = !state.show_bottom_panel;
        }
    });
    ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
}

pub fn render_bottom_panel_with_clicked_satellite(
    ui: &mut egui::Ui,
    store: &SatelliteStore,
    fetch_channels: &Option<Res<FetchChannels>>,
) {
    ui.horizontal(|ui| {
        ui.label(format!("Satellites: {}", store.items.len()));
        if let Some(_fetch) = fetch_channels {
            ui.separator();
            ui.label("TLE Fetcher: Active");
        } else {
            ui.separator();
            ui.colored_label(Color32::RED, "TLE Fetcher: Inactive");
        }

        // Display clicked satellite information by finding it in the store
        ui.separator();
        if let Some((norad, entry)) = store.items.iter().find(|(_, entry)| entry.is_clicked) {
            let satellite_name = entry.name.as_deref().unwrap_or("Unnamed");
            ui.colored_label(
                bevy_to_egui_color(entry.color),
                format!("Selected: {} (NORAD: {})", satellite_name, norad),
            );
        } else {
            ui.colored_label(Color32::GRAY, "Selected: None");
        }
    });
    ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
}

