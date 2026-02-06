//! TLE processing systems

use crate::satellite::SatelliteStore;
use crate::tle::parser::parse_tle_epoch_to_utc;
use crate::tle::types::{FetchChannels, FetchResultMsg, TleData};
use crate::ui::state::RightPanelUI;
use bevy::prelude::*;

/// System to drain fetch results and build SGP4 propagators
pub fn process_fetch_results_system(
    mut store: ResMut<SatelliteStore>,
    mut right_ui: ResMut<RightPanelUI>,
    fetch: Option<Res<FetchChannels>>,
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
                                s.propagator = Some(constants);
                            }
                            Err(e) => {
                                s.propagator = None;
                                s.error = Some(e.to_string());
                                eprintln!(
                                    "[SGP4] norad={} constants error: {}",
                                    norad,
                                    s.error.as_deref().unwrap()
                                );
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
                        }
                    }
                } else {
                    // Create a new SatEntry for this NORAD
                    use crate::satellite::SatEntry;
                    use bevy::prelude::Color;
                    let color = Color::hsl(store.next_color_hue, 0.8, 0.5);
                    store.next_color_hue = (store.next_color_hue + 137.5) % 360.0; // Golden angle for color diversity
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
                    s.error = Some(error);
                    s.tle = None;
                    s.propagator = None;
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
