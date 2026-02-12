//! TLE processing systems

use crate::satellite::components::{
    NoradId, PropagationError, Propagator, Satellite, SatelliteColor, SatelliteFlags,
    SatelliteGroupUrl, SatelliteName, TleComponent,
};
use crate::satellite::resources::{ColorHueCounter, GroupRegistry, NoradIndex};
use crate::tle::parser::parse_tle_epoch_to_utc;
use crate::tle::types::{FetchChannels, FetchResultMsg, TleData};
use crate::ui::state::RightPanelUI;
use bevy::prelude::*;

/// System to drain fetch results and build SGP4 propagators.
///
/// When a new satellite arrives, this system spawns a data-only entity
/// (no mesh/material). The `materialize_satellite_entities_system` adds
/// rendering components on the next frame.
pub fn process_fetch_results_system(
    mut norad_index: ResMut<NoradIndex>,
    mut color_hue: ResMut<ColorHueCounter>,
    mut right_ui: ResMut<RightPanelUI>,
    group_registry: Option<Res<GroupRegistry>>,
    fetch: Option<Res<FetchChannels>>,
    mut commands: Commands,
    // Queries for updating existing satellite entities
    mut sat_query: Query<(&mut SatelliteColor, Option<&mut SatelliteName>), With<Satellite>>,
) {
    let Some(fetch) = fetch else { return };
    let Ok(guard) = fetch.res_rx.lock() else {
        return;
    };
    while let Ok(msg) = guard.try_recv() {
        match msg {
            FetchResultMsg::Success {
                norad,
                name,
                line1,
                line2,
                epoch_utc,
                group,
            } => {
                let name_val = name.or_else(|| Some(format!("NORAD {}", norad)));
                let epoch = parse_tle_epoch_to_utc(&line1).unwrap_or(epoch_utc);
                let tle_data = TleData { epoch_utc: epoch };

                // Build SGP4 model
                let sgp4_result =
                    sgp4::Elements::from_tle(name_val.clone(), line1.as_bytes(), line2.as_bytes())
                        .map_err(|e| e.to_string())
                        .and_then(|elements| {
                            sgp4::Constants::from_elements(&elements).map_err(|e| e.to_string())
                        });

                if let Some(&entity) = norad_index.map.get(&norad) {
                    // ── Update existing entity ──
                    let mut ec = commands.entity(entity);
                    ec.insert(TleComponent(tle_data));
                    ec.remove::<PropagationError>();

                    if let Some(name) = &name_val {
                        ec.insert(SatelliteName(name.clone()));
                    }

                    match sgp4_result {
                        Ok(constants) => {
                            ec.insert(Propagator(constants));
                        }
                        Err(e) => {
                            ec.remove::<Propagator>();
                            eprintln!("[SGP4] norad={norad} error: {e}");
                            ec.insert(PropagationError);
                        }
                    }

                    // Sync color if this satellite just got assigned to a group
                    if let Some(group_url) = &group {
                        if let Some(registry) = &group_registry
                            && let Some(grp) = registry.groups.get(group_url)
                            && let Ok((mut color, _)) = sat_query.get_mut(entity)
                        {
                            color.0 = grp.color;
                        }
                        ec.insert(SatelliteGroupUrl(group_url.clone()));
                    }
                } else {
                    // ── Spawn new data-only entity ──
                    let (color, group_url) = resolve_color(&group, &group_registry, &mut color_hue);

                    let mut ec = commands.spawn((
                        Satellite,
                        NoradId(norad),
                        SatelliteColor(color),
                        SatelliteFlags::default(),
                        TleComponent(tle_data),
                    ));

                    if let Some(name) = &name_val {
                        ec.insert(SatelliteName(name.clone()));
                    }

                    match sgp4_result {
                        Ok(constants) => {
                            ec.insert(Propagator(constants));
                        }
                        Err(e) => {
                            eprintln!("[SGP4] norad={norad} error: {e}");
                            ec.insert(PropagationError);
                        }
                    }

                    if let Some(url) = &group_url {
                        ec.insert(SatelliteGroupUrl(url.clone()));
                    }

                    let entity = ec.id();
                    norad_index.map.insert(norad, entity);
                }
            }
            FetchResultMsg::Failure { norad, error } => {
                eprintln!(
                    "[TLE DISPATCH] received FAILURE for norad={}: {}",
                    norad, error
                );
                if let Some(&entity) = norad_index.map.get(&norad) {
                    commands
                        .entity(entity)
                        .remove::<TleComponent>()
                        .remove::<Propagator>()
                        .insert(PropagationError);
                } else {
                    eprintln!(
                        "[TLE DISPATCH] failure for unknown norad={} (not in index)",
                        norad
                    );
                }
            }
            FetchResultMsg::GroupDone { group, count } => {
                println!(
                    "[TLE DISPATCH] group={} done, {} satellites loaded",
                    group, count
                );
                right_ui.group_loading = false;
            }
            FetchResultMsg::GroupFailure { group, error } => {
                eprintln!("[TLE DISPATCH] group={} failed: {}", group, error);
                right_ui.group_loading = false;
                right_ui.error = Some(format!("Group fetch failed: {}", error));
            }
        }
    }
}

/// Determine color for a new satellite based on group registry or golden-angle hue.
fn resolve_color(
    group: &Option<String>,
    group_registry: &Option<Res<GroupRegistry>>,
    color_hue: &mut ColorHueCounter,
) -> (Color, Option<String>) {
    if let Some(group_url) = group {
        if let Some(registry) = group_registry
            && let Some(grp) = registry.groups.get(group_url)
        {
            return (grp.color, Some(group_url.clone()));
        }
        // Group URL provided but not found in registry — use golden angle
        let color = Color::hsl(color_hue.next_hue, 0.8, 0.5);
        color_hue.next_hue = (color_hue.next_hue + 137.5) % 360.0;
        (color, Some(group_url.clone()))
    } else {
        let color = Color::hsl(color_hue.next_hue, 0.8, 0.5);
        color_hue.next_hue = (color_hue.next_hue + 137.5) % 360.0;
        (color, None)
    }
}
