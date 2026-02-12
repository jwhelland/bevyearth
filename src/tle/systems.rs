//! TLE processing systems

use crate::satellite::resources::{GroupRegistry, NoradIndex};
use crate::satellite::{
    Propagator, PropagationError, SatelliteName, SatelliteStore, TleComponent,
};
use crate::tle::parser::parse_tle_epoch_to_utc;
use crate::tle::types::{FetchChannels, FetchResultMsg, TleData};
use crate::ui::state::RightPanelUI;
use bevy::prelude::*;

/// System to drain fetch results and build SGP4 propagators
pub fn process_fetch_results_system(
    mut store: ResMut<SatelliteStore>,
    mut right_ui: ResMut<RightPanelUI>,
    group_registry: Option<Res<GroupRegistry>>,
    norad_index: Option<Res<NoradIndex>>,
    fetch: Option<Res<FetchChannels>>,
    mut commands: Commands,
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
                if let Some(s) = store.items.get_mut(&norad) {
                    // clear previous error
                    s.error = None;
                    s.name = name.or_else(|| Some(format!("NORAD {}", norad)));
                    let epoch = parse_tle_epoch_to_utc(&line1).unwrap_or(epoch_utc);
                    s.tle = Some(TleData { epoch_utc: epoch });
                    // Build SGP4 model (sgp4 2.3.0): parse TLE -> Elements -> Constants
                    match sgp4::Elements::from_tle(
                        s.name.clone(),
                        line1.as_bytes(),
                        line2.as_bytes(),
                    ) {
                        Ok(elements) => match sgp4::Constants::from_elements(&elements) {
                            Ok(constants) => {
                                s.propagator = Some(constants.clone());

                                // Update entity components if entity exists
                                if let Some(norad_index) = &norad_index {
                                    if let Some(&entity) = norad_index.map.get(&norad) {
                                        commands.entity(entity).insert(Propagator(constants));
                                        commands.entity(entity).remove::<PropagationError>();
                                    }
                                }
                            }
                            Err(e) => {
                                s.propagator = None;
                                s.error = Some(e.to_string());
                                eprintln!(
                                    "[SGP4] norad={} constants error: {}",
                                    norad,
                                    s.error.as_deref().unwrap()
                                );

                                // Update entity with error
                                if let Some(norad_index) = &norad_index {
                                    if let Some(&entity) = norad_index.map.get(&norad) {
                                        commands.entity(entity).remove::<Propagator>();
                                        commands
                                            .entity(entity)
                                            .insert(PropagationError(e.to_string()));
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            s.propagator = None;
                            s.error = Some(e.to_string());
                            eprintln!(
                                "[SGP4] norad={} elements error: {}",
                                norad,
                                s.error.as_deref().unwrap()
                            );

                            // Update entity with error
                            if let Some(norad_index) = &norad_index {
                                if let Some(&entity) = norad_index.map.get(&norad) {
                                    commands.entity(entity).remove::<Propagator>();
                                    commands
                                        .entity(entity)
                                        .insert(PropagationError(e.to_string()));
                                }
                            }
                        }
                    }

                    // Update name and TLE components on entity
                    if let Some(norad_index) = &norad_index {
                        if let Some(&entity) = norad_index.map.get(&norad) {
                            if let Some(name) = &s.name {
                                commands.entity(entity).insert(SatelliteName(name.clone()));
                            }
                            if let Some(tle) = &s.tle {
                                commands.entity(entity).insert(TleComponent(tle.clone()));
                            }
                        }
                    }
                } else {
                    // Create a new SatEntry for this NORAD
                    use crate::satellite::SatEntry;
                    use bevy::prelude::Color;

                    // Determine color and group_url based on whether this is a group load
                    let (color, group_url) = if let Some(group_url) = group {
                        // Try to get color from group registry
                        if let Some(registry) = &group_registry {
                            if let Some(group) = registry.groups.get(&group_url) {
                                (group.color, Some(group_url))
                            } else {
                                // Group not found, fall back to golden angle
                                let color = Color::hsl(store.next_color_hue, 0.8, 0.5);
                                store.next_color_hue = (store.next_color_hue + 137.5) % 360.0;
                                (color, Some(group_url))
                            }
                        } else {
                            // No registry available, fall back to golden angle
                            let color = Color::hsl(store.next_color_hue, 0.8, 0.5);
                            store.next_color_hue = (store.next_color_hue + 137.5) % 360.0;
                            (color, Some(group_url))
                        }
                    } else {
                        // Not loading as part of a group, use golden angle
                        let color = Color::hsl(store.next_color_hue, 0.8, 0.5);
                        store.next_color_hue = (store.next_color_hue + 137.5) % 360.0;
                        (color, None)
                    };

                    let epoch = parse_tle_epoch_to_utc(&line1).unwrap_or(epoch_utc);
                    let name_val = name.clone().or_else(|| Some(format!("NORAD {}", norad)));
                    let propagator = sgp4::Elements::from_tle(
                        name_val.clone(),
                        line1.as_bytes(),
                        line2.as_bytes(),
                    )
                    .ok()
                    .and_then(|elements| sgp4::Constants::from_elements(&elements).ok());
                    let entry = SatEntry {
                        name: name_val.clone(),
                        color,
                        entity: None,
                        tle: Some(TleData { epoch_utc: epoch }),
                        propagator,
                        error: None,
                        show_ground_track: false,
                        show_trail: false,
                        is_clicked: false,
                        group_url,
                    };
                    store.items.insert(norad, entry);
                }
            }
            FetchResultMsg::Failure { norad, error } => {
                eprintln!(
                    "[TLE DISPATCH] received FAILURE for norad={}: {}",
                    norad, error
                );
                if let Some(s) = store.items.get_mut(&norad) {
                    // keep existing name if any; record error and clear models
                    s.error = Some(error.clone());
                    s.tle = None;
                    s.propagator = None;

                    // Update entity components
                    if let Some(norad_index) = &norad_index {
                        if let Some(&entity) = norad_index.map.get(&norad) {
                            commands.entity(entity).remove::<TleComponent>();
                            commands.entity(entity).remove::<Propagator>();
                            commands
                                .entity(entity)
                                .insert(PropagationError(error.clone()));
                        }
                    }
                } else {
                    eprintln!(
                        "[TLE DISPATCH] failure for unknown norad={} (not in store)",
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
